mod highlight;
mod html;
mod identifier;
mod lexer;
mod number;
mod scanner;
mod stream;
mod string;
mod token;
mod tree;
mod trivia;

pub use highlight::*;
pub(crate) use identifier::keyword_from_identifier;
pub use lexer::*;
pub use stream::*;
pub use tree::{decode_html_entities, decode_html_entity};

#[cfg(test)]
mod tests;
