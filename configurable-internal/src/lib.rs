mod assumptions;
mod dispatch;
mod include;

use std::collections::HashMap;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Item, ItemImpl, ItemMod, parse2};

use crate::assumptions::{extract_assumptions, has_assumptions};
use crate::dispatch::{FunctionVariant, generate_dispatch, process_impl_block};
use crate::include::expand_includes;

/// The core logic function.
///
/// This is exposed as a library function so that a proc-macro crate can call it
/// with specific configuration (e.g. "assumptions" vs "kernelversion").
pub fn __internal_configurable(
    item: TokenStream,
    macro_name: &str,
    mod_name: &str,
    attr_name: &str,
    package_name: &str,
) -> TokenStream {
    let mut item_mod = parse2::<ItemMod>(item)
        .unwrap_or_else(|_| panic!("#[{}] must be applied to a module", macro_name));

    if let Some((_, ref mut items)) = item_mod.content {
        if let Err(e) = expand_includes(items, mod_name) {
            return e.into_compile_error();
        }

        let mut new_items = Vec::new();

        let mut fn_groups: HashMap<String, Vec<FunctionVariant>> = HashMap::new();
        let mut fn_order: Vec<String> = Vec::new();

        let mut impl_groups: HashMap<String, ItemImpl> = HashMap::new();
        let mut impl_order: Vec<String> = Vec::new();

        for item in items.drain(..) {
            match item {
                Item::Fn(mut func) => {
                    if has_assumptions(&func.attrs, attr_name) {
                        let name = func.sig.ident.to_string();
                        let assumptions = extract_assumptions(&mut func.attrs, attr_name);

                        if !fn_groups.contains_key(&name) {
                            fn_order.push(name.clone());
                        }

                        fn_groups.entry(name).or_default().push(FunctionVariant {
                            item: Item::Fn(func),
                            assumptions,
                        });
                    } else {
                        new_items.push(Item::Fn(func));
                    }
                }
                Item::Impl(mut impl_block) => {
                    let key = impl_group_key(&impl_block);

                    if let Some(existing) = impl_groups.get_mut(&key) {
                        existing.items.append(&mut impl_block.items);
                    } else {
                        impl_order.push(key.clone());
                        impl_groups.insert(key, impl_block);
                    }
                }
                _ => new_items.push(item),
            }
        }

        for name in fn_order {
            let variants = fn_groups.remove(&name).unwrap();
            let dispatcher = generate_dispatch(&name, variants, package_name);
            new_items.extend(dispatcher);
        }

        for key in impl_order {
            let impl_block = impl_groups.remove(&key).unwrap();
            let processed_blocks = process_impl_block(impl_block, attr_name, package_name);
            new_items.extend(processed_blocks);
        }

        *items = new_items;
    }

    TokenStream::from(quote! { #item_mod })
}

fn impl_group_key(impl_block: &ItemImpl) -> String {
    let trait_str = if let Some((_, path, _)) = &impl_block.trait_ {
        path.to_token_stream().to_string()
    } else {
        String::new()
    };
    let type_str = impl_block.self_ty.to_token_stream().to_string();
    let generics_str = impl_block.generics.to_token_stream().to_string();
    format!("{}|{}|{}", trait_str, type_str, generics_str)
}
