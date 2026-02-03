//! NOTE: This crate is in early development.

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, punctuated::Punctuated, Attribute, Block, FnArg, Ident, Item, ItemFn,
    ItemMod, Meta, Pat, PatIdent, PatType, ReturnType, Token,
};

struct FunctionData {
    block: Block,
    kernelv: Option<TokenStream>,
}

struct FunctionGroup {
    id: Ident,
    params: Punctuated<FnArg, Token![,]>,
    functions: Vec<FunctionData>,
    match_arms: Vec<TokenStream>,
    return_type: ReturnType,
    lazy: Ident,
}

struct FunctionSupergroup(Vec<FunctionGroup>);

impl FunctionData {
    fn new(block: Block, kernelv: Option<TokenStream>) -> FunctionData {
        FunctionData { block, kernelv }
    }

    fn new_from_itemfn(item_fn: ItemFn) -> FunctionData {
        FunctionData::new(
            get_block_from_item_fn(&item_fn),
            FunctionData::get_kernel_version_from_item_fn(&item_fn),
        )
    }

    fn build_function(
        &self,
        params: Punctuated<FnArg, Token![,]>,
        mut function_name: Ident,
        n: i32,
        return_type: ReturnType,
    ) -> TokenStream {
        let block_ = &self.block;
        let params_ = params.clone().into_iter();

        function_name = Ident::new(&format!("{}{}", function_name, n), Span::call_site().into());

        quote! {
            fn #function_name (#(#params_),*) #return_type #block_
        }
    }

    fn get_kernel_version_from_item_fn(item_fn: &ItemFn) -> Option<TokenStream> {
        let tokens = if let Meta::List(list) = &item_fn.attrs[0].meta {
            list.tokens.clone()
        } else {
            return None;
        };

        //(Platform, Feature)
        let mut kernv: Vec<(TokenStream, TokenStream)> = Vec::new();
        let mut iter = tokens.into_iter();

        while let Some(platform_token) = iter.next() {
            let platform: TokenStream = platform_token.into();

            let _punct = iter.next();

            if let Some(feature_token) = iter.next() {
                let feature: TokenStream = feature_token.into();
                kernv.push((platform, feature));
            } else {
                break;
            }

            let _sep = iter.next();
        }

        if kernv.len() == 0 {
            return None;
        }

        let mut tokens = TokenStream::new();
        let mut tokens_len = kernv.len();
        for (platform, feature) in kernv {
            let platform_str = platform.to_string();
            let platform_lit = syn::LitStr::new(&platform_str, proc_macro2::Span::call_site());

            if tokens_len == 1 {
                tokens.extend(quote! {
                    (#platform_lit.to_string(),
                        Arc::new(#feature) as Arc<dyn Feature>
                    )
                });
            } else {
                tokens.extend(quote! {
                    (#platform_lit.to_string(), Arc::new(#feature)
                        as Arc<dyn Feature>
                    ),
                })
            }

            tokens_len -= 1;
        }

        Some(quote! {
            HashMap::from([#tokens])
        })
    }
}

impl FunctionGroup {
    fn new(
        id: Ident,
        params: Punctuated<FnArg, Token![,]>,
        functions: Vec<FunctionData>,
        match_arms: Vec<TokenStream>,
        return_type: ReturnType,
    ) -> FunctionGroup {
        FunctionGroup {
            lazy: Ident::new(
                &format!("{}_lazy_ref", &id.to_string()),
                Span::call_site().into(),
            ),
            id,
            params,
            functions,
            match_arms,
            return_type,
        }
    }

    fn build_arms(&mut self, function_name: Ident, params_name: Vec<Ident>, n: i32) {
        self.match_arms = FunctionGroup::create_arms(function_name, params_name, n);
    }

    fn create_arms(function_name: Ident, params_name: Vec<Ident>, n: i32) -> Vec<TokenStream> {
        let mut arms: Vec<TokenStream> = Vec::new();

        for i in 0..n {
            let new_function_name =
                Ident::new(&format!("{}{}", function_name, i), Span::call_site().into());
            let expr = quote! {
                #i => #new_function_name (#(#params_name),*)
            };

            arms.push(expr);
        }

        let fallback_name = Ident::new(&format!("{}0", function_name), Span::call_site().into());
        let expr = quote! {
            _ => #fallback_name (#(#params_name),*)
        };

        arms.push(expr);

        arms
    }

    fn build_group_dispatch(&self) -> TokenStream {
        let id_ = &self.id;
        let params_ = self.params.clone().into_iter();
        let arms_ = &self.match_arms;
        let return_type_ = &self.return_type;
        let lazy_ = &self.lazy;

        let dispatch = quote! {
            pub fn #id_ (#(#params_),*) #return_type_ {
                match *#lazy_ {
                    #(#arms_),*
                }
            }
        };

        dispatch
    }

    fn build_resolve(&self) -> TokenStream {
        let lazy_ = &self.lazy;

        let platforms_list: Vec<TokenStream> = {
            let mut platforms: Vec<TokenStream> = Vec::new();

            for f in &self.functions {
                match &f.kernelv {
                    None => continue,
                    Some(kv) => {
                        platforms.push(kv.clone());
                    }
                }
            }

            platforms
        };

        let resolve = quote! {
            lazy_static! {
                static ref #lazy_ : i32 = {
                    resolve(vec![#(#platforms_list),*])
                };
            }
        };

        resolve
    }

    fn get_params_name_from_itemfn(item_fn: &ItemFn) -> Vec<Ident> {
        item_fn
            .sig
            .inputs
            .iter()
            .filter_map(|arg| {
                let FnArg::Typed(PatType { pat, .. }) = arg else {
                    return None;
                };
                let Pat::Ident(PatIdent { ident, .. }) = &**pat else {
                    return None;
                };
                Some(ident.clone())
            })
            .collect()
    }
}

impl FunctionSupergroup {
    fn build_supergroup(item_mod: &ItemMod, idents: Vec<Ident>) -> FunctionSupergroup {
        let mut fsg = FunctionSupergroup(Vec::new());

        for ident in idents {
            let ident_ = ident.clone();
            let mut fg = FunctionGroup::new(
                ident,
                Punctuated::new(),
                Vec::new(),
                Vec::new(),
                ReturnType::Default,
            );
            let mut params_names: Vec<Ident> = Vec::new();

            if let Some(tuple) = &item_mod.content {
                let content_array = tuple.clone().1;

                content_array.into_iter().for_each(|item| match item {
                    Item::Fn(item_fn) if item_fn.sig.ident == ident_ => {
                        fg.functions.push(FunctionData::new_from_itemfn(item_fn.clone()));
                        fg.params = get_inputs_from_itemfn(&item_fn);
                        params_names = FunctionGroup::get_params_name_from_itemfn(&item_fn);
                        fg.return_type = get_return_type_from_itemfn(&item_fn);
                    }
                    _ => {}
                })
            }

            if fg.functions.len() == 0 {
                continue;
            }

            fg.build_arms(fg.id.clone(), params_names, fg.functions.len() as i32);

            fsg.0.push(fg);
        }

        fsg
    }

    fn build_all_match_arms(&self) -> Vec<TokenStream> {
        let mut arms: Vec<TokenStream> = Vec::new();

        for fg in &self.0 {
            arms.push(fg.build_group_dispatch());
        }

        arms
    }

    fn build_all_functions(&self) -> Vec<TokenStream> {
        let mut functions: Vec<TokenStream> = Vec::new();

        for fg in &self.0 {
            for (i, f) in fg.functions.iter().enumerate() {
                functions.push(f.build_function(
                    fg.params.clone(),
                    fg.id.clone(),
                    i as i32,
                    fg.return_type.clone(),
                ));
            }
        }

        functions
    }

    fn build_all_resolves(&self) -> Vec<TokenStream> {
        let mut resolves: Vec<TokenStream> = Vec::new();

        for fg in &self.0 {
            resolves.push(fg.build_resolve());
        }

        resolves
    }
}

fn get_return_type_from_itemfn(item_fn: &ItemFn) -> ReturnType {
    item_fn.sig.output.clone()
}

fn get_function_ids_from_attr(attr: proc_macro::TokenStream) -> Vec<Ident> {
    let mut ids: Vec<Ident> = Vec::new();

    attr.into_iter().for_each(|token| {
        if let proc_macro::TokenTree::Ident(ident) = token {
            ids.push(Ident::new(&ident.to_string(), Span::call_site()));
        }
    });

    ids
}

fn get_contents_from_itemmod(item_mod: &ItemMod) -> Vec<TokenStream> {
    fn is_assumptions(attr: &Attribute) -> bool {
        attr.path().is_ident("assumptions")
    }

    let items = if let Some((_, items)) = &item_mod.content {
        items
    } else {
        return Vec::new();
    };

    items
        .iter()
        .filter(|item| match item {
            Item::Fn(item_fn) => !item_fn.attrs.iter().any(|attr| is_assumptions(attr)),
            _ => true,
        })
        .map(|item| item.to_token_stream())
        .collect()
}

fn get_block_from_item_fn(item_fn: &ItemFn) -> Block {
    *item_fn.block.clone()
}

fn get_inputs_from_itemfn(item_fn: &ItemFn) -> Punctuated<FnArg, Token![,]> {
    item_fn.sig.inputs.clone()
}

fn get_name_from_itemmod(item_mod: &ItemMod) -> Ident {
    item_mod.ident.clone()
}

/// This macro should be used in a `mod` declaration. It takes as attributes
/// the names of the functions declared in the module and builds a multiple
/// dispatch for each one of them based on the specified platform features.
///
/// # Example
/// ## Declaration
/// ```
/// #[configurable(add, multiply)]
/// mod math_operations {
///     #[assumptions]
///     pub fn add(a: i32, b: i32) -> i32 {
///         println!("Using fallback add");
///         a + b
///     }
///
///     #[assumptions(cpu_simd=AVX512)]
///     pub fn add(a: i32, b: i32) -> i32 {
///         println!("Using AVX512 optimized add");
///         a + b
///     }
///
///     #[assumptions]
///     pub fn multiply(a: i32, b: i32) -> i32 {
///         println!("Using fallback multiply");
///         a * b
///     }
///
///     #[assumptions(acc_backend=CUDA10)]
///     pub fn multiply(a: i32, b: i32) -> i32 {
///         println!("Using CUDA10 optimized multiply");
///         a * b
///     }
/// }
/// ```
/// ## Calling
/// ```
/// let sum = math_operations::add(5, 10);
/// let product = math_operations::multiply(5, 10);
/// ```
#[proc_macro_attribute]
pub fn configurable(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item_mod = parse_macro_input!(item as ItemMod);
    let fn_ids = get_function_ids_from_attr(attr);
    let mod_name = get_name_from_itemmod(&item_mod);

    let fsg = FunctionSupergroup::build_supergroup(&item_mod, fn_ids);

    let dispatches = fsg.build_all_match_arms();
    let functions = fsg.build_all_functions();
    let resolves = fsg.build_all_resolves();
    let contents = get_contents_from_itemmod(&item_mod);

    let expanded = quote! {
        mod #mod_name {
            use platform_aware_features::*;
            use lazy_static::lazy_static;
            use std::{collections::HashMap, sync::Arc};

            #(#contents)*

            #(#resolves)*

            #(#dispatches)*

            #(#functions)*
        }
    };

    eprintln!(
        "\x1b[93m@INFO | Generated the following module:\x1b[00m\n {}",
        proc_macro::TokenStream::from(expanded.clone())
    );

    proc_macro::TokenStream::from(expanded)
}



