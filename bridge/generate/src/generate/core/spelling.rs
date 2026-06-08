use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Return one identifier.
pub(super) fn ident(name: &str) -> proc_macro2::Ident {
    format_ident!("{name}")
}

/// Convert one PascalCase or snake_case name to lower camel.
pub(super) fn lower_camel(name: &str) -> String {
    let name = to_snake(name);
    let mut output = String::new();
    let mut uppercase = false;

    for character in name.chars() {
        if character == '_' {
            uppercase = true;
        } else if uppercase {
            output.extend(character.to_uppercase());
            uppercase = false;
        } else {
            output.push(character);
        }
    }

    output
}

/// Convert one PascalCase name to snake_case.
pub(super) fn to_snake(name: &str) -> String {
    let mut output = String::new();

    for (index, character) in name.chars().enumerate() {
        if character.is_uppercase() && index > 0 {
            output.push('_');
        }
        output.extend(character.to_lowercase());
    }

    output
}

/// Render documentation attributes.
pub(super) fn render_docs(lines: &[String]) -> TokenStream {
    let lines = lines.iter().map(|line| format!(" {line}"));

    quote!(#(#[doc = #lines])*)
}
