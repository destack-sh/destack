use proc_macro::TokenStream;

mod service;

/// Declare one statically typed TS++ RPC service.
#[proc_macro_attribute]
pub fn service(attribute: TokenStream, input: TokenStream) -> TokenStream {
    service::expand(attribute, input)
}
