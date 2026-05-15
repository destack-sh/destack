mod attribute;
mod constant;
mod error;
mod function;
mod instruction;
mod key;
mod module;
mod parser;
mod trivia;
mod r#type;
mod value;

pub use error::*;
pub use parser::*;

#[cfg(test)]
mod tests;
