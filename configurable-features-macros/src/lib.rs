use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use syn::{
    parse_macro_input, spanned::Spanned, Attribute, FnArg, ImplItem, Item, ItemFn, ItemImpl,
    ItemMod, ItemMacro, Meta, Signature, Visibility,
};

#[proc_macro_attribute]
pub fn configurable(attr: TokenStream, item: TokenStream) -> TokenStream {
    let _ = attr;
    
    let mut item_mod = parse_macro_input!(item as ItemMod);

    if let Err(e) = expand_includes(&mut item_mod) {
        return e.into_compile_error().into();
    }

    if let Some((_, ref mut items)) = item_mod.content {
        let mut new_items = Vec::new();
        
        let mut fn_groups: HashMap<String, Vec<FunctionVariant>> = HashMap::new();
        
        for item in items.drain(..) {
            match item {
                Item::Fn(mut func) => {
                    if has_assumptions(&func.attrs) {
                        let name = func.sig.ident.to_string();
                        let assumptions = extract_assumptions(&mut func.attrs);
                        fn_groups.entry(name).or_default().push(FunctionVariant {
                            item: Item::Fn(func),
                            assumptions,
                        });
                    } else {
                        new_items.push(Item::Fn(func));
                    }
                }
                Item::Impl(mut impl_block) => {
                    process_impl_block(&mut impl_block);
                    new_items.push(Item::Impl(impl_block));
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

fn process_impl_block(impl_block: &mut ItemImpl) {
    let mut method_groups: HashMap<String, Vec<ImplItem>> = HashMap::new();
    let mut methods_assumptions: HashMap<String, Vec<Option<proc_macro2::TokenStream>>> = HashMap::new();
    let mut other_items = Vec::new();

    for item in impl_block.items.drain(..) {
        if let ImplItem::Fn(mut method) = item {
            if has_assumptions(&method.attrs) {
                let name = method.sig.ident.to_string();
                let assumptions = extract_assumptions(&mut method.attrs);
                
                methods_assumptions.entry(name.clone()).or_default().push(assumptions);
                method_groups.entry(name).or_default().push(ImplItem::Fn(method));
            } else {
                other_items.push(ImplItem::Fn(method));
            }
        } else {
            other_items.push(item);
        }
    }

    for (name, mut methods) in method_groups {
        let assumptions_list = methods_assumptions.remove(&name).unwrap();
        
        let master_sig = if let ImplItem::Fn(m) = &methods[0] {
            m.sig.clone()
        } else { unreachable!() };
        
        let vis = if let ImplItem::Fn(m) = &methods[0] {
            m.vis.clone()
        } else { unreachable!() };

        let mut variant_idents = Vec::new();
        for (i, method_item) in methods.iter_mut().enumerate() {
            if let ImplItem::Fn(m) = method_item {
                let new_name = format_ident!("{}_variant_{}", name, i);
                m.sig.ident = new_name.clone();
                variant_idents.push(new_name);
            }
            other_items.push(method_item.clone());
        }

        let dispatcher = generate_impl_dispatcher(&name, &master_sig, &vis, &variant_idents, &assumptions_list);
        other_items.push(dispatcher);
    }

    impl_block.items = other_items;
}

fn expand_includes(item_mod: &mut ItemMod) -> syn::Result<()> {
    if let Some((_, ref mut items)) = item_mod.content {
        let mut i = 0;
        while i < items.len() {
            let should_expand = if let Item::Macro(mac) = &items[i] {
                mac.mac.path.is_ident("configurable")
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

    let platforms_vec = build_platforms_vec(&assumption_tokens);
    
    let args = args_from_sig(&master_sig);
    let mut match_arms = Vec::new();
    
    for (idx, variant_ident) in variant_names.iter().enumerate() {
        let idx_lit = syn::LitInt::new(&idx.to_string(), proc_macro2::Span::call_site());
        match_arms.push(quote! {
            #idx_lit => #variant_ident(#args)
        });
    }

    let fallback_ident = &variant_names[0];
    match_arms.push(quote! {
        _ => #fallback_ident(#args)
    });

    let ident = format_ident!("{}", original_name);
    let generics = &master_sig.generics;
    let inputs = &master_sig.inputs;
    let output = &master_sig.output;
    let where_clause = &master_sig.generics.where_clause;
    
    let dispatcher = quote! {
        #master_vis fn #ident #generics (#inputs) #output #where_clause {
            use platform_aware_features::*;
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

    items.push(syn::parse2(dispatcher).unwrap());
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
    
    let mut match_arms = Vec::new();
    for (idx, variant_ident) in variants.iter().enumerate() {
        let idx_lit = syn::LitInt::new(&idx.to_string(), proc_macro2::Span::call_site());
        match_arms.push(quote! {
            #idx_lit => Self::#variant_ident(#args)
        });
    }
    let fallback = &variants[0];
    match_arms.push(quote! { _ => Self::#fallback(#args) });

    let ident = format_ident!("{}", name);
    let generics = &sig.generics;
    let inputs = &sig.inputs;
    let output = &sig.output;
    let where_clause = &sig.generics.where_clause;

    let has_receiver = inputs.iter().any(|arg| matches!(arg, FnArg::Receiver(_)));
    
    let call_prefix = if has_receiver {
        quote! { self. }
    } else {
        quote! { Self:: }
    };

    let mut method_match_arms = Vec::new();
    for (idx, variant_ident) in variants.iter().enumerate() {
        let idx_lit = syn::LitInt::new(&idx.to_string(), proc_macro2::Span::call_site());
        method_match_arms.push(quote! {
            #idx_lit => #call_prefix #variant_ident(#args)
        });
    }
    method_match_arms.push(quote! { _ => #call_prefix #fallback(#args) });

    let item = quote! {
        #vis fn #ident #generics (#inputs) #output #where_clause {
            use platform_aware_features::*;
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


fn has_assumptions(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("assumptions") || attr.path().is_ident("kernelversion"))
}

fn extract_assumptions(attrs: &mut Vec<Attribute>) -> Option<proc_macro2::TokenStream> {
    let idx = attrs.iter().position(|attr| attr.path().is_ident("assumptions") || attr.path().is_ident("kernelversion"))?;
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
                std::sync::Arc::new(#value) as std::sync::Arc<dyn platform_aware_features::Feature>
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
