use proc_macro::TokenStream;

mod schema;

/// Derive a Destack serialization schema.
#[proc_macro_derive(Schema)]
pub fn derive_schema(input: TokenStream) -> TokenStream {
    schema::expand(input)
}
