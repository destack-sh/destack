mod diagnostic;

use proc_macro::TokenStream;

/// Derive one provider diagnostic enum.
#[proc_macro_derive(Diagnostic, attributes(diagnostic))]
pub fn diagnostic(input: TokenStream) -> TokenStream {
    diagnostic::diagnostic_impl(input)
}
