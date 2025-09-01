#![feature(default_field_values)]

mod argument;
mod r#as;
mod binary;
mod block;
mod call;
mod expression;
mod function;
mod identifier;
mod keyword;
mod literal;
mod r#loop;
mod r#match;
mod parameter;
mod path;
mod pattern;
mod statement;
mod r#struct;
mod r#trait;
mod r#try;
mod r#type;
mod unary;
mod union;
mod using;

pub mod ast;
pub mod error;
pub mod format;
pub mod parse;

pub use ast::*;
pub use error::*;
pub use format::*;
pub use identifier::*;
pub use keyword::*;
pub use parse::*;

#[cfg(test)]
mod tests;
