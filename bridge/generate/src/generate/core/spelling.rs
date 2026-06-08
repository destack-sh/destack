use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Return one identifier.
pub(super) fn ident(name: &str) -> proc_macro2::Ident {
    if is_rust_keyword(name) {
        format_ident!("r#{name}")
    } else {
        format_ident!("{name}")
    }
}

/// Return whether one name is a Rust keyword.
fn is_rust_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "async"
            | "await"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
    )
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

/// Convert one snake_case or PascalCase name to UpperCamelCase.
pub(super) fn upper_camel(name: &str) -> String {
    let name = to_snake(name);
    let mut output = String::new();
    let mut uppercase = true;

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
pub(in crate::generate) fn to_snake(name: &str) -> String {
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
