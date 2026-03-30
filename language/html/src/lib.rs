#![feature(default_field_values)]

pub mod format;
pub mod lex;
pub mod parse;
pub mod print;
pub mod tree;

pub use format::{HtmlFormatOptions, format_document, format_fragment};
pub use parse::{Parser, parse_fragment, parse_html};
pub use tree::*;
