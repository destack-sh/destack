mod highlight;
mod html_entities;
mod lex;
mod lexer;
mod stream;
mod trivia;

pub use highlight::*;
pub use lex::*;
pub use lexer::*;
pub use stream::*;

#[cfg(test)]
mod tests;
