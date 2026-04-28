#![feature(default_field_values)]
#![feature(if_let_guard)]
#![allow(clippy::too_many_arguments)]

pub mod annotation;
pub mod call;
pub mod chain;
pub mod collection;
mod context;
pub mod declaration;
pub mod expression;
pub mod file;
mod jsdoc;
pub mod operator;
pub mod tree;

pub use context::*;
pub use declaration::statement::statement_list;
pub use file::*;

#[cfg(test)]
mod tests;
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use tests::*;
