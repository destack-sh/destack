use proc_macro::TokenStream;

mod reflect;

/// Derive Destack reflection metadata.
#[proc_macro_derive(Reflect)]
pub fn derive_reflect(input: TokenStream) -> TokenStream {
    reflect::expand(input)
}
