mod traversal;

use proc_macro::TokenStream;

/// Derive a structural type visit and fold.
#[proc_macro_derive(TypeFold)]
pub fn type_fold(input: TokenStream) -> TokenStream {
    traversal::expand_type(input)
}

/// Derive one structural node fold.
#[proc_macro_derive(NodeFold)]
pub fn node_fold(input: TokenStream) -> TokenStream {
    traversal::expand_node(input)
}

/// Derive one structural selection visit.
#[proc_macro_derive(InstanceKeyVisit)]
pub fn selection_visit(input: TokenStream) -> TokenStream {
    traversal::expand_selection(input)
}
