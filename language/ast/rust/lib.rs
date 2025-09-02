#![feature(default_field_values)]

mod argument;
mod r#as;
mod binary;
mod block;
mod call;
mod doc;
mod expression;
mod function;
mod keyword;
mod literal;
mod r#loop;
mod r#match;
mod parameter;
mod path;
mod path_pool;
mod pattern;
mod statement;
mod r#struct;
mod r#trait;
mod tree;
mod r#try;
mod r#type;
mod unary;
mod union;
mod using;

pub mod error;
pub mod format;
pub mod node;
pub mod parse;

pub use error::*;
pub use format::*;
pub use keyword::*;
pub use node::*;
pub use parse::*;
pub use path_pool::*;
pub use tree::*;

pub use destack_language_arena::{StringId, StringPool};

#[cfg(test)]
mod tests;
