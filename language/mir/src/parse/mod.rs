//! MIR text format parser.
//!
//! Parses the text representation of MIR back into a NodeTree.
//! Used for testing roundtrips and debugging.

mod error;
mod lexer;
mod parser;
mod token;

pub use error::*;
pub use lexer::*;
pub use parser::*;
pub use token::*;

#[cfg(test)]
mod tests;
