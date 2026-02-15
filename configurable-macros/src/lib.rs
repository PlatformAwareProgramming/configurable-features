use proc_macro::TokenStream;
use configurable_internal::__internal_configurable;

#[proc_macro_attribute]
pub fn configurable(_: TokenStream, item: TokenStream) -> TokenStream {
    __internal_configurable(item.into(), "configurable", "assumptions").into()
}
