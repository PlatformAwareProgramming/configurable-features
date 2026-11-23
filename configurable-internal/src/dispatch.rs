use std::collections::HashMap;
use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::{FnArg, GenericParam, ImplItem, Item, ItemImpl, Pat, Signature, Visibility};

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
    attr_name: &str,
    package_name: &str,
) -> syn::Result<Vec<Item>> {
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
        attr_name,
        package_name,
        CallStyle::Free,
    )?;

    items.push(syn::parse2(dispatcher_tokens)?);
    Ok(items)
}

pub(crate) fn process_impl_block(
    mut impl_block: ItemImpl,
    impl_idx: usize,
    attr_name: &str,
    package_name: &str,
) -> syn::Result<Vec<Item>> {
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

    let is_trait_impl = impl_block.trait_.is_some();
    let helper_trait_ident = format_ident!("__ConfigurableVariants_{}", impl_idx);
    let (_, ty_generics, _) = impl_block.generics.split_for_impl();
    let helper_ty_generics = ty_generics.to_token_stream();

    let mut variants_to_add: Vec<ImplItem> = Vec::new();
    let mut helper_trait_sigs: Vec<Signature> = Vec::new();

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

                if is_trait_impl {
                    m.vis = Visibility::Inherited;
                    helper_trait_sigs.push(sanitize_trait_method_sig(&m.sig));
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
            attr_name,
            package_name,
            if is_trait_impl {
                Some((&helper_trait_ident, &helper_ty_generics))
            } else {
                None
            },
        )?;
        other_items.push(dispatcher);
    }

    impl_block.items = other_items;
    let mut result_items = vec![Item::Impl(impl_block.clone())];

    if let Some((_, ref trait_path, _)) = impl_block.trait_ {
        // Trait Implementation (impl Trait for Type):
        // Emit variants in a private local helper trait (supertrait-bounded by Trait)
        // instead of an inherent `impl Type`, preserving both cross-crate extensibility
        // (no orphan-rule E0116) and collision-free k-methods across multiple traits.
        if !variants_to_add.is_empty() {
            let (impl_generics, ty_generics, where_clause) = impl_block.generics.split_for_impl();
            let self_ty = &impl_block.self_ty;

            let helper_trait_tokens = quote! {
                #[doc(hidden)]
                #[allow(non_camel_case_types, dead_code)]
                trait #helper_trait_ident #impl_generics : #trait_path #where_clause {
                    #(#helper_trait_sigs;)*
                }
            };
            result_items.push(syn::parse2(helper_trait_tokens)?);

            let helper_impl_tokens = quote! {
                #[doc(hidden)]
                #[allow(non_camel_case_types, dead_code)]
                impl #impl_generics #helper_trait_ident #ty_generics for #self_ty #where_clause {
                    #(#variants_to_add)*
                }
            };
            result_items.push(syn::parse2(helper_impl_tokens)?);
        }
    } else {
        // Inherent Implementation (impl Type)
        if let Item::Impl(ref mut original) = result_items[0] {
            original.items.extend(variants_to_add);
        }
    }

    Ok(result_items)
}

enum CallStyle<'a> {
    Free,
    Inherent {
        has_receiver: bool,
    },
    HelperTrait {
        trait_ident: &'a Ident,
        ty_generics: &'a TokenStream,
        has_receiver: bool,
    },
}

fn generate_impl_dispatcher(
    name: &str,
    sig: &Signature,
    vis: &Visibility,
    variants: &[Ident],
    assumptions: &[Option<TokenStream>],
    attr_name: &str,
    package_name: &str,
    helper_trait: Option<(&Ident, &TokenStream)>,
) -> syn::Result<ImplItem> {
    let has_receiver = sig
        .inputs
        .iter()
        .any(|arg| matches!(arg, FnArg::Receiver(_)));

    let call_style = match helper_trait {
        Some((trait_ident, ty_generics)) => CallStyle::HelperTrait {
            trait_ident,
            ty_generics,
            has_receiver,
        },
        None => CallStyle::Inherent { has_receiver },
    };

    let item_tokens = build_dispatcher_fn_tokens(
        name,
        sig,
        vis,
        variants,
        assumptions,
        attr_name,
        package_name,
        call_style,
    )?;

    syn::parse2(item_tokens)
}

/// Shared generator for both standalone k-function dispatchers and impl k-method dispatchers.
fn build_dispatcher_fn_tokens(
    name: &str,
    sig: &Signature,
    vis: &Visibility,
    variants: &[Ident],
    assumptions: &[Option<TokenStream>],
    attr_name: &str,
    package_name: &str,
    call_style: CallStyle<'_>,
) -> syn::Result<TokenStream> {
    let fallback_idx = assumptions
        .iter()
        .position(is_empty_assumption)
        .ok_or_else(|| {
            syn::Error::new(
                sig.ident.span(),
                format!(
                    "kernel `{}` must declare a fallback version with `#[{}]` (no platform assumptions)",
                    name, attr_name
                ),
            )
        })?;

    let platforms_vec = build_platforms_vec(assumptions);
    let args = args_from_sig(sig);
    let turbofish = method_turbofish(sig);

    let await_call = if sig.asyncness.is_some() {
        quote! { .await }
    } else {
        quote! {}
    };

    let make_call = |variant_ident: &Ident| match &call_style {
        CallStyle::Free => quote! { #variant_ident #turbofish (#args) #await_call },
        CallStyle::Inherent { has_receiver } => {
            if *has_receiver {
                quote! { self.#variant_ident #turbofish (#args) #await_call }
            } else {
                quote! { Self::#variant_ident #turbofish (#args) #await_call }
            }
        }
        CallStyle::HelperTrait {
            trait_ident,
            ty_generics,
            has_receiver,
        } => {
            let ufcs_args = if *has_receiver {
                if args.is_empty() {
                    quote! { self }
                } else {
                    quote! { self, #args }
                }
            } else {
                quote! { #args }
            };
            quote! { <Self as #trait_ident #ty_generics>::#variant_ident #turbofish (#ufcs_args) #await_call }
        }
    };

    let mut match_arms = Vec::new();
    for (idx, variant_ident) in variants.iter().enumerate() {
        let idx_lit = syn::LitInt::new(&idx.to_string(), proc_macro2::Span::call_site());
        let call = make_call(variant_ident);
        match_arms.push(quote! {
            #idx_lit => #call
        });
    }

    let fallback_call = make_call(&variants[fallback_idx]);
    match_arms.push(quote! {
        _ => #fallback_call
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

    Ok(quote! {
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
    })
}

fn method_turbofish(sig: &Signature) -> TokenStream {
    let where_str = sig
        .generics
        .where_clause
        .as_ref()
        .map(|w| w.to_token_stream().to_string())
        .unwrap_or_default();

    let mut non_lifetime_args = Vec::new();
    let mut early_bound_lifetimes = Vec::new();

    for param in &sig.generics.params {
        match param {
            GenericParam::Lifetime(lt) => {
                let lt_name = lt.lifetime.to_string();
                if !lt.bounds.is_empty() || where_str.contains(&lt_name) {
                    let lifetime = &lt.lifetime;
                    early_bound_lifetimes.push(quote! { #lifetime });
                }
            }
            GenericParam::Type(ty) => {
                let ident = &ty.ident;
                non_lifetime_args.push(quote! { #ident });
            }
            GenericParam::Const(c) => {
                let ident = &c.ident;
                non_lifetime_args.push(quote! { #ident });
            }
        }
    }

    if non_lifetime_args.is_empty() {
        return quote! {};
    }

    quote! { ::<#(#early_bound_lifetimes,)* #(#non_lifetime_args),*> }
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
        match arg {
            FnArg::Typed(pat_type) => {
                if let Pat::Ident(pat_ident) = &mut *pat_type.pat {
                    pat_ident.mutability = None;
                }
            }
            FnArg::Receiver(receiver) => {
                if receiver.reference.is_none() {
                    receiver.mutability = None;
                }
            }
        }
    }
    cleaned
}

fn sanitize_trait_method_sig(sig: &Signature) -> Signature {
    let mut cleaned = sig.clone();
    for (idx, arg) in cleaned.inputs.iter_mut().enumerate() {
        match arg {
            FnArg::Typed(pat_type) => match &mut *pat_type.pat {
                Pat::Ident(pat_ident) => {
                    pat_ident.mutability = None;
                    pat_ident.by_ref = None;
                    pat_ident.subpat = None;
                }
                other => {
                    let ident = format_ident!("__arg_{}", idx);
                    *other = syn::parse_quote!(#ident);
                }
            },
            FnArg::Receiver(receiver) => {
                if receiver.reference.is_none() {
                    receiver.mutability = None;
                }
            }
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

    #[test]
    fn test_missing_fallback_emits_compile_error_for_k_function_and_k_method() {
        let fn_input: TokenStream = quote! {
            mod kernels {
                #[kernelversion(cpu_simd = AVX512)]
                pub fn f(x: i64) -> i64 { x }
            }
        };
        let fn_out = crate::__internal_configurable(
            fn_input,
            "platformaware",
            "platform",
            "kernelversion",
            "platform_aware",
        )
        .to_string();
        assert!(fn_out.contains("compile_error"));
        assert!(fn_out.contains("must declare a fallback version"));

        let method_input: TokenStream = quote! {
            mod kernels {
                impl MyTrait for MyType {
                    #[kernelversion(cpu_simd = AVX512)]
                    fn f(&self, x: i64) -> i64 { x }
                }
            }
        };
        let method_out = crate::__internal_configurable(
            method_input,
            "platformaware",
            "platform",
            "kernelversion",
            "platform_aware",
        )
        .to_string();
        assert!(method_out.contains("compile_error"));
        assert!(method_out.contains("must declare a fallback version"));
    }

    #[test]
    fn test_trait_impl_uses_helper_trait_instead_of_inherent_impl() {
        let input: TokenStream = quote! {
            mod kernels {
                impl MyTrait for Vec<f64> {
                    #[kernelversion]
                    fn compute(&self, x: i64) -> i64 { x }

                    #[kernelversion(cpu_simd = AVX512)]
                    fn compute(&self, x: i64) -> i64 { x + 1 }
                }
            }
        };
        let out = crate::__internal_configurable(
            input,
            "platformaware",
            "platform",
            "kernelversion",
            "platform_aware",
        )
        .to_string();
        assert!(out.contains("trait __ConfigurableVariants_0 : MyTrait"));
        assert!(out.contains("impl __ConfigurableVariants_0 for Vec < f64 >"));
        assert!(!out.contains("impl Vec < f64 >"));
    }
}
