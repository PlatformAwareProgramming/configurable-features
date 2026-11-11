use std::fs;
use std::path::{self, PathBuf};
use syn::{ImplItem, Item, ItemImpl, spanned::Spanned};

use crate::assumptions::has_assumptions;

pub(crate) fn expand_includes(items: &mut Vec<Item>, mod_name: &str) -> syn::Result<()> {
    let mut i = 0;
    while i < items.len() {
        if let Item::Impl(impl_block) = &mut items[i] {
            expand_impl_includes(impl_block, mod_name)?;
        }

        // Support inline platform submodules: #[<mod_name>] mod sub { ... }
        let is_inline_platform_mod = if let Item::Mod(submod) = &items[i] {
            submod.content.is_some() && has_assumptions(&submod.attrs, mod_name)
        } else {
            false
        };

        if is_inline_platform_mod {
            let item = items.remove(i);
            if let Item::Mod(submod) = item
                && let Some((_, sub_items)) = submod.content
            {
                for new_item in sub_items.into_iter().rev() {
                    items.insert(i, new_item);
                }
                continue;
            }
        }

        // Support external file/module macro inclusion: <mod_name>!(pmodule)
        let should_expand = if let Item::Macro(mac) = &items[i] {
            mac.mac.path.is_ident(mod_name)
        } else {
            false
        };

        if should_expand {
            let item = items.remove(i);
            if let Item::Macro(mac) = item {
                let current_module_path = mac.mac.span().unwrap().file();
                let current_directory = path::Path::new(&current_module_path)
                    .parent()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let pmodule_name: syn::Ident = mac.mac.parse_body()?;
                let pmod_str = pmodule_name.to_string();

                let candidate_rs = if current_directory.is_empty() {
                    format!("{}.rs", pmod_str)
                } else {
                    format!("{}/{}.rs", current_directory, pmod_str)
                };
                let candidate_mod_rs = if current_directory.is_empty() {
                    format!("{}/mod.rs", pmod_str)
                } else {
                    format!("{}/{}/mod.rs", current_directory, pmod_str)
                };

                let (loaded_path, file_content) = read_included_file_candidates(
                    &[candidate_rs, candidate_mod_rs],
                    mac.mac.span(),
                )?;
                let file_ast: syn::File = syn::parse_str(&file_content).map_err(|e| {
                    syn::Error::new(
                        mac.mac.span(),
                        format!("Failed to parse included file {:?}: {}", loaded_path, e),
                    )
                })?;

                for new_item in file_ast.items.into_iter().rev() {
                    items.insert(i, new_item);
                }
                continue;
            }
        }
        i += 1;
    }
    Ok(())
}

fn expand_impl_includes(impl_block: &mut ItemImpl, mod_name: &str) -> syn::Result<()> {
    let mut i = 0;
    while i < impl_block.items.len() {
        let should_expand = if let ImplItem::Macro(mac) = &impl_block.items[i] {
            mac.mac.path.is_ident(mod_name)
        } else {
            false
        };

        if should_expand {
            let item = impl_block.items.remove(i);
            if let ImplItem::Macro(mac) = item {
                let path_str: syn::LitStr = mac.mac.parse_body()?;
                let (_, file_content) =
                    read_included_file_candidates(&[path_str.value()], mac.mac.span())?;

                // Wrap the file contents in a dummy implementation so it can be parsed as a collection of ImplItem elements
                let dummy_code = format!("impl Dummy {{ {} }}", file_content);
                let dummy_impl: syn::ItemImpl = syn::parse_str(&dummy_code).map_err(|e| {
                    syn::Error::new(
                        mac.mac.span(),
                        format!(
                            "Failed to parse included impl items from {:?}: {}",
                            path_str.value(),
                            e
                        ),
                    )
                })?;

                for new_item in dummy_impl.items.into_iter().rev() {
                    impl_block.items.insert(i, new_item);
                }
                continue;
            }
        }
        i += 1;
    }
    Ok(())
}

fn read_included_file_candidates(
    rel_paths: &[String],
    span: proc_macro2::Span,
) -> syn::Result<(String, String)> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let mut paths_to_try = Vec::new();

    for path in rel_paths {
        // 1. Try directly (works for absolute or workspace-relative paths)
        paths_to_try.push(PathBuf::from(path));

        // 2. Try relative to the crate root (CARGO_MANIFEST_DIR)
        let mut p1 = PathBuf::from(&manifest_dir);
        p1.push(path);
        paths_to_try.push(p1);

        // 3. Try relative to the `src` folder (where users typically place their files)
        let mut p2 = PathBuf::from(&manifest_dir);
        p2.push("src");
        p2.push(path);
        paths_to_try.push(p2);
    }

    for p in &paths_to_try {
        if let Ok(content) = fs::read_to_string(p) {
            return Ok((p.to_string_lossy().into_owned(), content));
        }
    }

    let tried_paths: Vec<String> = paths_to_try
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    Err(syn::Error::new(
        span,
        format!(
            "Failed to read file {:?}. Tried locations: {:?}",
            rel_paths, tried_paths
        ),
    ))
}
