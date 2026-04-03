#![feature(default_field_values)]

pub mod format;
pub mod lex;
pub mod parse;
pub mod print;
pub mod tree;

pub use format::{HtmlFormatOptions, format_document, format_fragment};
pub use parse::{
    ParseContextName, Parser, ParserOptions, parse_fragment, parse_fragment_with_options,
    parse_html, parse_html_with_options,
};
pub use tree::*;

#[cfg(test)]
mod tests;
