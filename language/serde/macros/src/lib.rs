use proc_macro::TokenStream;

mod reflect;
mod section;

/// Derive TS++ reflection metadata.
#[proc_macro_derive(Reflect, attributes(reflect))]
pub fn derive_reflect(input: TokenStream) -> TokenStream {
    reflect::expand(input)
}

/// Derive immutable section entry support.
#[proc_macro_derive(SectionEntry)]
pub fn derive_section_entry(input: TokenStream) -> TokenStream {
    section::expand(input)
}
