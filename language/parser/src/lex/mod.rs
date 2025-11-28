mod highlight;
mod html_entities;
mod lex;
mod lexer;
mod memchr;

pub use highlight::*;
pub use lex::*;
pub use lexer::*;

#[cfg(test)]
mod tests;
