mod comment;
mod highlight;
mod html;
mod identifier;
mod lexer;
mod number;
mod probe;
mod scanner;
mod stream;
mod string;
mod token;
mod tokenizer;
mod tree;

pub use highlight::*;
pub(crate) use identifier::classify_keyword;
pub use lexer::*;
pub(crate) use probe::*;
pub use stream::*;
pub(crate) use tokenizer::Tokenizer;
pub use tree::{decode_html_entities, decode_html_entity};

#[cfg(test)]
mod tests;
