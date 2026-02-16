use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use syn::{
    Attribute, FnArg, ImplItem, Item, ItemFn, ItemImpl, ItemMacro, ItemMod, Meta,
    Signature, Visibility, parse2, spanned::Spanned
};

/// The core logic function.
/// 
/// This is exposed as a library function so that a proc-macro crate can call it
/// with specific configuration (e.g. "assumptions" vs "kernelversion").
pub fn __internal_configurable(item: TokenStream, macro_name: &str, attr_name: &str) -> TokenStream {    
    let mut item_mod = parse2::<ItemMod>(item).expect("Must be applied to a module");

    if let Err(e) = expand_includes(&mut item_mod, macro_name) {
        return e.into_compile_error().into();
    }

    if let Some((_, ref mut items)) = item_mod.content {
        let mut new_items = Vec::new();
        
        let mut fn_groups: HashMap<String, Vec<FunctionVariant>> = HashMap::new();
        
        for item in items.drain(..) {
            match item {
                Item::Fn(mut func) => {
                    if has_assumptions(&func.attrs, attr_name) {
                        let name = func.sig.ident.to_string();
                        let assumptions = extract_assumptions(&mut func.attrs, attr_name);
                        fn_groups.entry(name).or_default().push(FunctionVariant {
                            item: Item::Fn(func),
                            assumptions,
                        });
                    } else {
                        new_items.push(Item::Fn(func));
                    }
                }
                Item::Impl(impl_block) => {
                    let processed_blocks = process_impl_block(impl_block, attr_name);
                    new_items.extend(processed_blocks);
                }
                _ => new_items.push(item),
            }
        }

        for (name, variants) in fn_groups {
            let dispatcher = generate_dispatch(&name, variants);
            new_items.extend(dispatcher);
        }
        
        *items = new_items;
    }

    TokenStream::from(quote! { #item_mod })
}

struct FunctionVariant {
    item: Item, 
    assumptions: Option<proc_macro2::TokenStream>,
}

fn process_impl_block(mut impl_block: ItemImpl, attr_name: &str) -> Vec<Item> {
    let mut method_groups: HashMap<String, Vec<ImplItem>> = HashMap::new();
    let mut methods_assumptions: HashMap<String, Vec<Option<proc_macro2::TokenStream>>> = HashMap::new();
    let mut other_items = Vec::new();

    for item in impl_block.items.drain(..) {
        if let ImplItem::Fn(mut method) = item {
            if has_assumptions(&method.attrs, attr_name) {
                let name = method.sig.ident.to_string();
                let assumptions = extract_assumptions(&mut method.attrs, attr_name);
                
                methods_assumptions.entry(name.clone()).or_default().push(assumptions);
                method_groups.entry(name).or_default().push(ImplItem::Fn(method));
            } else {
                other_items.push(ImplItem::Fn(method));
            }
        } else {
            other_items.push(item);
        }
    }

    let mut variants_to_add: Vec<ImplItem> = Vec::new();

    for (name, mut methods) in method_groups {
        let assumptions_list = methods_assumptions
            .remove(&name)
            .expect(&format!("Mismatched assumptions for methoded: {}", name));
        
        let master_sig = if let ImplItem::Fn(m) = &methods[0] { m.sig.clone() } else { unreachable!() };
        let vis = if let ImplItem::Fn(m) = &methods[0] { m.vis.clone() } else { unreachable!() };

        let mut variant_idents = Vec::new();
        for (i, method_item) in methods.iter_mut().enumerate() {
            if let ImplItem::Fn(m) = method_item {
                let new_name = format_ident!("{}_variant_{}", name, i);
                m.sig.ident = new_name.clone();
                variant_idents.push(new_name);

                // hide variants from docs
                if impl_block.trait_.is_some() {
                     m.vis = Visibility::Public(syn::token::Pub::default());
                     m.attrs.push(syn::parse_quote!(#[doc(hidden)]));
                } else {
                    m.attrs.push(syn::parse_quote!(#[doc(hidden)]));
                }
            }
            variants_to_add.push(method_item.clone());
        }

        let dispatcher = generate_impl_dispatcher(&name, &master_sig, &vis, &variant_idents, &assumptions_list);
        other_items.push(dispatcher);
    }

    impl_block.items = other_items;
    let mut result_items = vec![Item::Impl(impl_block.clone())];

    if impl_block.trait_.is_some() {
        // Trait Implementation (impl Trait for Type)
        if !variants_to_add.is_empty() {
            let mut inherent_impl = impl_block.clone();
            inherent_impl.trait_ = None; // Remove "Trait for"
            inherent_impl.items = variants_to_add;
            result_items.push(Item::Impl(inherent_impl));
        }
    } else {
        // Inherent Implementation (impl Type)
        if let Item::Impl(ref mut original) = result_items[0] {
            original.items.extend(variants_to_add);
        }
    }

    result_items
}

fn expand_includes(item_mod: &mut ItemMod, macro_name: &str) -> syn::Result<()> {
    if let Some((_, ref mut items)) = item_mod.content {
        let mut i = 0;
        while i < items.len() {
            let should_expand = if let Item::Macro(mac) = &items[i] {
                mac.mac.path.is_ident(macro_name)
            } else {
                false
            };

            if should_expand {
                let item = items.remove(i);
                if let Item::Macro(mac) = item {
                    let path_str: syn::LitStr = mac.mac.parse_body()?;
                    let file_content = read_included_file(&path_str.value())?;
                    let file_ast: syn::File = syn::parse_str(&file_content)?;
                    
                    for new_item in file_ast.items.into_iter().rev() {
                        items.insert(i, new_item);
                    }
                    continue; 
                }
            }
            i += 1;
        }
    }
    Ok(())
}

fn read_included_file(path: &str) -> syn::Result<String> {
    let mut file_path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    file_path.push(path);

    fs::read_to_string(&file_path).map_err(|e| {
        syn::Error::new(proc_macro2::Span::call_site(), format!("Failed to read file {:?}: {}", file_path, e))
    })
}

fn generate_dispatch(original_name: &str, variants: Vec<FunctionVariant>) -> Vec<Item> {
    let mut items = Vec::new();
    let mut variant_names = Vec::new();
    let mut assumption_tokens = Vec::new();

    let master_sig = if let Item::Fn(f) = &variants[0].item { f.sig.clone() } else { panic!("Not a function") };
    let master_vis = if let Item::Fn(f) = &variants[0].item { f.vis.clone() } else { panic!("Not a function") };

    for (i, variant) in variants.into_iter().enumerate() {
        let mut func = if let Item::Fn(f) = variant.item { f } else { panic!() };
        let new_ident = format_ident!("{}_variant_{}", original_name, i);
        func.sig.ident = new_ident.clone();
        
        items.push(Item::Fn(func));
        variant_names.push(new_ident);
        assumption_tokens.push(variant.assumptions);
    }

    let fallback_idx = assumption_tokens.iter()
        .position(|t| is_empty_assumption(t))
        .unwrap_or(0); 

    let platforms_vec = build_platforms_vec(&assumption_tokens);
    let args = args_from_sig(&master_sig);
    
    let await_call = if master_sig.asyncness.is_some() { quote!{.await} } else { quote!{} };

    let mut match_arms = Vec::new();
    for (idx, variant_ident) in variant_names.iter().enumerate() {
        let idx_lit = syn::LitInt::new(&idx.to_string(), proc_macro2::Span::call_site());
        match_arms.push(quote! {
            #idx_lit => #variant_ident(#args) #await_call
        });
    }

    let fallback_ident = &variant_names[fallback_idx];
    match_arms.push(quote! {
        _ => #fallback_ident(#args) #await_call
    });

    let ident = format_ident!("{}", original_name);
    let generics = &master_sig.generics;
    let inputs = &master_sig.inputs;
    let output = &master_sig.output;
    let where_clause = &master_sig.generics.where_clause;
    
    let constness = &master_sig.constness;
    let asyncness = &master_sig.asyncness;
    let unsafety = &master_sig.unsafety;
    let abi = &master_sig.abi;

    let dispatcher = quote! {
        #master_vis #constness #asyncness #unsafety #abi fn #ident #generics (#inputs) #output #where_clause {
            use std::sync::Arc;
            use std::collections::HashMap;
            use lazy_static::lazy_static;

            lazy_static! {
                static ref SELECTED_VARIANT: i32 = {
                    let variants = vec![#platforms_vec];
                    resolve(variants)
                };
            }

            match *SELECTED_VARIANT {
                #(#match_arms),*
            }
        }
    };

    items.push(syn::parse2(dispatcher).expect("Failed to parse dispatcher"));
    items
}

fn generate_impl_dispatcher(
    name: &str, 
    sig: &Signature, 
    vis: &Visibility, 
    variants: &[proc_macro2::Ident], 
    assumptions: &[Option<proc_macro2::TokenStream>]
) -> ImplItem {
    let platforms_vec = build_platforms_vec(assumptions);
    let args = args_from_sig(sig);
    
    let fallback_idx = assumptions.iter()
        .position(|t| is_empty_assumption(t))
        .unwrap_or(0);

    let await_call = if sig.asyncness.is_some() { quote!{.await} } else { quote!{} };

    let has_receiver = sig.inputs.iter().any(|arg| matches!(arg, FnArg::Receiver(_)));
    let call_prefix = if has_receiver { quote! { self. } } else { quote! { Self:: } };

    let mut method_match_arms = Vec::new();
    for (idx, variant_ident) in variants.iter().enumerate() {
        let idx_lit = syn::LitInt::new(&idx.to_string(), proc_macro2::Span::call_site());
        method_match_arms.push(quote! {
            #idx_lit => #call_prefix #variant_ident(#args) #await_call
        });
    }
    
    let fallback = &variants[fallback_idx];
    method_match_arms.push(quote! { _ => #call_prefix #fallback(#args) #await_call });

    let ident = format_ident!("{}", name);
    let generics = &sig.generics;
    let inputs = &sig.inputs;
    let output = &sig.output;
    let where_clause = &sig.generics.where_clause;
    
    let constness = &sig.constness;
    let asyncness = &sig.asyncness;
    let unsafety = &sig.unsafety;
    let abi = &sig.abi;

    let item = quote! {
        #vis #constness #asyncness #unsafety #abi fn #ident #generics (#inputs) #output #where_clause {
            use std::sync::Arc;
            use std::collections::HashMap;
            use lazy_static::lazy_static;

            lazy_static! {
                static ref SELECTED_VARIANT: i32 = {
                    let variants = vec![#platforms_vec];
                    resolve(variants)
                };
            }

            match *SELECTED_VARIANT {
                #(#method_match_arms),*
            }
        }
    };
    
    syn::parse2(item).expect("Failed to parse impl dispatcher")
}

fn is_empty_assumption(tokens: &Option<proc_macro2::TokenStream>) -> bool {
    match tokens {
        None => true,
        Some(t) => t.is_empty(),
    }
}

fn has_assumptions(attrs: &[Attribute], attr_name: &str) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident(attr_name))
}

fn extract_assumptions(attrs: &mut Vec<Attribute>, attr_name: &str) -> Option<proc_macro2::TokenStream> {
    let idx = attrs.iter().position(|attr| attr.path().is_ident(attr_name))?;
    let attr = attrs.remove(idx);
    
    if let Meta::List(list) = attr.meta {
         return Some(list.tokens);
    }
    Some(quote! {})
}

fn build_platforms_vec(assumptions_list: &[Option<proc_macro2::TokenStream>]) -> proc_macro2::TokenStream {
    let mut array_items = Vec::new();
    
    for tokens_opt in assumptions_list {
        if let Some(tokens) = tokens_opt {
             array_items.push(transform_tokens_to_hashmap(tokens.clone()));
        } else {
            array_items.push(quote! { HashMap::new() });
        }
    }
    
    quote! { #(#array_items),* }
}

fn transform_tokens_to_hashmap(tokens: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    use syn::parse::Parser;
    use syn::{punctuated::Punctuated, MetaNameValue, Token};

    if tokens.is_empty() {
        return quote! { std::collections::HashMap::new() };
    }

    let parser = Punctuated::<MetaNameValue, Token![,]>::parse_terminated;
    
    let args = match parser.parse2(tokens) {
        Ok(args) => args,
        Err(e) => return e.into_compile_error(),
    };

    let mut pairs = Vec::new();
    
    for nv in args {
        let key = nv.path;
        let value = nv.value;

        let key_str = key.into_token_stream().to_string().replace(" ", "");

        pairs.push(quote! {
            (
                #key_str.to_string(),
                std::sync::Arc::new(#value) as std::sync::Arc<dyn Feature>
            )
        });
    }

    quote! {
        std::collections::HashMap::from([
            #(#pairs),*
        ])
    }
}

fn args_from_sig(sig: &Signature) -> proc_macro2::TokenStream {
    let args: Vec<_> = sig.inputs.iter().filter_map(|arg| {
        match arg {
            FnArg::Typed(pat) => Some(&pat.pat),
            FnArg::Receiver(_) => None,
        }
    }).collect();
    quote! { #(#args),* }
}
