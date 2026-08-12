mod fold;

use proc_macro::TokenStream;

/// Derive one structural type fold.
#[proc_macro_derive(TypeFold)]
pub fn type_fold(input: TokenStream) -> TokenStream {
    fold::expand(input)
}
