use std::collections::HashMap;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{FnArg, ImplItem, Item, ItemImpl, Pat, Signature, Visibility};

use crate::assumptions::{
    build_platforms_vec, extract_assumptions, has_assumptions, is_empty_assumption,
};

pub(crate) struct FunctionVariant {
    pub(crate) item: Item,
    pub(crate) assumptions: Option<TokenStream>,
}

pub(crate) fn generate_dispatch(
    original_name: &str,
    variants: Vec<FunctionVariant>,
    package_name: &str,
) -> Vec<Item> {
    let mut items = Vec::new();
    let mut variant_names = Vec::new();
    let mut assumption_tokens = Vec::new();

    let master_sig = if let Item::Fn(f) = &variants[0].item {
        f.sig.clone()
    } else {
        panic!("Not a function")
    };
    let master_vis = if let Item::Fn(f) = &variants[0].item {
        f.vis.clone()
    } else {
        panic!("Not a function")
    };

    for (i, variant) in variants.into_iter().enumerate() {
        let mut func = if let Item::Fn(f) = variant.item {
            f
        } else {
            panic!()
        };
        let new_ident = format_ident!("{}_variant_{}", original_name, i);
        func.sig.ident = new_ident.clone();

        items.push(Item::Fn(func));
        variant_names.push(new_ident);
        assumption_tokens.push(variant.assumptions);
    }

    let dispatcher_tokens = build_dispatcher_fn_tokens(
        original_name,
        &master_sig,
        &master_vis,
        &variant_names,
        &assumption_tokens,
        package_name,
        quote! {},
    );

    items.push(syn::parse2(dispatcher_tokens).expect("Failed to parse function dispatcher"));
    items
}

pub(crate) fn process_impl_block(
    mut impl_block: ItemImpl,
    attr_name: &str,
    package_name: &str,
) -> Vec<Item> {
    let mut method_groups: HashMap<String, Vec<ImplItem>> = HashMap::new();
    let mut methods_assumptions: HashMap<String, Vec<Option<TokenStream>>> = HashMap::new();
    let mut other_items = Vec::new();
    let mut method_order = Vec::new();

    for item in impl_block.items.drain(..) {
        if let ImplItem::Fn(mut method) = item {
            if has_assumptions(&method.attrs, attr_name) {
                let name = method.sig.ident.to_string();
                let assumptions = extract_assumptions(&mut method.attrs, attr_name);

                if !method_groups.contains_key(&name) {
                    method_order.push(name.clone());
                }

                methods_assumptions
                    .entry(name.clone())
                    .or_default()
                    .push(assumptions);
                method_groups
                    .entry(name)
                    .or_default()
                    .push(ImplItem::Fn(method));
            } else {
                other_items.push(ImplItem::Fn(method));
            }
        } else {
            other_items.push(item);
        }
    }

    let mut variants_to_add: Vec<ImplItem> = Vec::new();

    for name in method_order {
        let mut methods = method_groups.remove(&name).unwrap();
        let assumptions_list = methods_assumptions
            .remove(&name)
            .unwrap_or_else(|| panic!("Mismatched assumptions for method: {}", name));

        let master_sig = if let ImplItem::Fn(m) = &methods[0] {
            m.sig.clone()
        } else {
            unreachable!()
        };
        let vis = if let ImplItem::Fn(m) = &methods[0] {
            m.vis.clone()
        } else {
            unreachable!()
        };

        let mut variant_idents = Vec::new();
        for (i, method_item) in methods.iter_mut().enumerate() {
            if let ImplItem::Fn(m) = method_item {
                let new_name = format_ident!("{}_variant_{}", name, i);
                m.sig.ident = new_name.clone();
                variant_idents.push(new_name);

                if impl_block.trait_.is_some() {
                    m.vis = Visibility::Public(syn::token::Pub::default());
                }
                m.attrs.push(syn::parse_quote!(#[doc(hidden)]));
            }
            variants_to_add.push(method_item.clone());
        }

        let dispatcher = generate_impl_dispatcher(
            &name,
            &master_sig,
            &vis,
            &variant_idents,
            &assumptions_list,
            package_name,
        );
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

fn generate_impl_dispatcher(
    name: &str,
    sig: &Signature,
    vis: &Visibility,
    variants: &[Ident],
    assumptions: &[Option<TokenStream>],
    package_name: &str,
) -> ImplItem {
    let has_receiver = sig
        .inputs
        .iter()
        .any(|arg| matches!(arg, FnArg::Receiver(_)));
    let call_prefix = if has_receiver {
        quote! { self. }
    } else {
        quote! { Self:: }
    };

    let item_tokens = build_dispatcher_fn_tokens(
        name,
        sig,
        vis,
        variants,
        assumptions,
        package_name,
        call_prefix,
    );

    syn::parse2(item_tokens).expect("Failed to parse impl dispatcher")
}

/// Shared generator for both standalone k-function dispatchers and impl k-method dispatchers.
fn build_dispatcher_fn_tokens(
    name: &str,
    sig: &Signature,
    vis: &Visibility,
    variants: &[Ident],
    assumptions: &[Option<TokenStream>],
    package_name: &str,
    call_prefix: TokenStream,
) -> TokenStream {
    let platforms_vec = build_platforms_vec(assumptions);
    let args = args_from_sig(sig);

    let fallback_idx = assumptions
        .iter()
        .position(is_empty_assumption)
        .unwrap_or(0);

    let await_call = if sig.asyncness.is_some() {
        quote! { .await }
    } else {
        quote! {}
    };

    let mut match_arms = Vec::new();
    for (idx, variant_ident) in variants.iter().enumerate() {
        let idx_lit = syn::LitInt::new(&idx.to_string(), proc_macro2::Span::call_site());
        match_arms.push(quote! {
            #idx_lit => #call_prefix #variant_ident(#args) #await_call
        });
    }

    let fallback = &variants[fallback_idx];
    match_arms.push(quote! {
        _ => #call_prefix #fallback(#args) #await_call
    });

    let ident = format_ident!("{}", name);
    let generics = &sig.generics;
    let inputs = sanitize_dispatcher_inputs(&sig.inputs);
    let output = &sig.output;
    let where_clause = &sig.generics.where_clause;

    let constness = &sig.constness;
    let asyncness = &sig.asyncness;
    let unsafety = &sig.unsafety;
    let abi = &sig.abi;
    let pn = <TokenStream as std::str::FromStr>::from_str(package_name)
        .expect("invalid package name");

    quote! {
        #vis #constness #asyncness #unsafety #abi fn #ident #generics (#inputs) #output #where_clause {
            use std::sync::{Arc, LazyLock};
            use std::collections::HashMap;
            use #pn::{resolve, Feature};

            static SELECTED_VARIANT: LazyLock<i32> = LazyLock::new(|| {
                let variants = vec![#platforms_vec];
                resolve(variants)
            });

            match *SELECTED_VARIANT {
                #(#match_arms),*
            }
        }
    }
}

pub(crate) fn args_from_sig(sig: &Signature) -> TokenStream {
    let args: Vec<TokenStream> = sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            FnArg::Typed(pat_type) => match &*pat_type.pat {
                Pat::Ident(pat_ident) => {
                    let ident = &pat_ident.ident;
                    Some(quote! { #ident })
                }
                other_pat => Some(quote! { #other_pat }),
            },
            FnArg::Receiver(_) => None,
        })
        .collect();
    quote! { #(#args),* }
}

fn sanitize_dispatcher_inputs(
    inputs: &syn::punctuated::Punctuated<FnArg, syn::Token![,]>,
) -> syn::punctuated::Punctuated<FnArg, syn::Token![,]> {
    let mut cleaned = inputs.clone();
    for arg in cleaned.iter_mut() {
        if let FnArg::Typed(pat_type) = arg
            && let Pat::Ident(pat_ident) = &mut *pat_type.pat
        {
            pat_ident.mutability = None;
        }
    }
    cleaned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_from_sig_strips_mut_keyword_from_arguments() {
        let sig: Signature = syn::parse_quote!(fn compute(&mut self, mut x: i64, y: i64) -> i64);
        let args = args_from_sig(&sig);
        assert_eq!(args.to_string(), "x , y");
    }
}
