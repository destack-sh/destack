mod fold;
mod selections;

use proc_macro::TokenStream;

/// Derive one structural type fold.
#[proc_macro_derive(TypeFold)]
pub fn type_fold(input: TokenStream) -> TokenStream {
    fold::expand(input)
}

/// Derive one structural selection walk.
#[proc_macro_derive(WalkSelections)]
pub fn walk_selections(input: TokenStream) -> TokenStream {
    selections::expand(input)
}
