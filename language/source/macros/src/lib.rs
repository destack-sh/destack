mod image;

use proc_macro::TokenStream;

/// Derive the `AdaptImage` trait for one structural type.
#[proc_macro_derive(AdaptImage)]
pub fn derive_adapt_image(input: TokenStream) -> TokenStream {
    image::derive_adapt_image(input)
}
