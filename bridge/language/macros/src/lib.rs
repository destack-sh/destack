use proc_macro::TokenStream;

/// Mark one bridge language DTO.
#[proc_macro_attribute]
pub fn bridge(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    item
}
