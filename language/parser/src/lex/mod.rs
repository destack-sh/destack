mod cache;
mod highlight;
mod html_entities;
mod lex;
mod lexer;
mod scanner;
mod trivia;

pub(crate) use cache::keyword_from_identifier;
pub use highlight::*;
pub use lex::*;
pub use lexer::*;

#[cfg(test)]
mod tests;
