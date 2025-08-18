use proc_macro::TokenStream;

// Minimal no-deps macro placeholder. For now it just passes through input.
// This allows future ergonomic annotations without adding heavy dependencies.
#[proc_macro]
pub fn destack_command(input: TokenStream) -> TokenStream {
    input
}
