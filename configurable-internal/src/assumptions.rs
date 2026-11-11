use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::parse::Parser;
use syn::{Attribute, Meta, MetaNameValue, Token, punctuated::Punctuated};

pub(crate) fn is_empty_assumption(tokens: &Option<TokenStream>) -> bool {
    match tokens {
        None => true,
        Some(t) => t.is_empty(),
    }
}

pub(crate) fn has_assumptions(attrs: &[Attribute], attr_name: &str) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident(attr_name))
}

pub(crate) fn extract_assumptions(
    attrs: &mut Vec<Attribute>,
    attr_name: &str,
) -> Option<TokenStream> {
    let idx = attrs.iter().position(|attr| attr.path().is_ident(attr_name))?;
    let attr = attrs.remove(idx);

    if let Meta::List(list) = attr.meta {
        return Some(list.tokens);
    }
    Some(quote! {})
}

pub(crate) fn build_platforms_vec(assumptions_list: &[Option<TokenStream>]) -> TokenStream {
    let mut array_items = Vec::new();

    for tokens_opt in assumptions_list {
        if let Some(tokens) = tokens_opt {
            array_items.push(transform_tokens_to_hashmap(tokens.clone()));
        } else {
            array_items.push(quote! { std::collections::HashMap::new() });
        }
    }

    quote! { #(#array_items),* }
}

pub(crate) fn transform_tokens_to_hashmap(tokens: TokenStream) -> TokenStream {
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

        let key_str = key.into_token_stream().to_string().replace(' ', "");

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
