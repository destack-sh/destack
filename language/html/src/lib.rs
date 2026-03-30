#![feature(default_field_values)]

/// Format HTML trees into pretty printed output.
pub mod format;

/// Tokenize HTML source.
pub mod lex;

/// Parse HTML source into the owned tree.
pub mod parse;

/// Print HTML trees without reformatting them.
pub mod print;

/// Store the owned HTML tree representation.
pub mod tree;

pub use format::{HtmlFormatOptions, format_document, format_fragment};
pub use parse::{Parser, parse_fragment, parse_html};
pub use tree::*;
