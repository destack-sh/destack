mod bridge;

use proc_macro::TokenStream;

/// Mark one bridge language DTO.
#[proc_macro_attribute]
pub fn bridge(attribute: TokenStream, item: TokenStream) -> TokenStream {
    bridge::expand(attribute, item)
}
