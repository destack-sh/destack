use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn generated(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn partial(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn custom(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
