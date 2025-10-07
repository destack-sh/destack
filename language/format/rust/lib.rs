#![feature(default_field_values)]
#![feature(if_let_guard)]

pub mod annotation;
pub mod argument;
pub mod block;
pub mod context;
pub mod definition;
pub mod r#enum;
pub mod expression;
pub mod function;
pub mod identifier;
pub mod r#if;
pub mod implement;
pub mod r#let;
pub mod literal;
pub mod r#match;
pub mod path;
pub mod pattern;
pub mod r#struct;
pub mod r#trait;
pub mod r#try;
pub mod union;
pub mod r#use;
pub mod r#where;
pub mod with;

pub use block::{EmptyBlockWithInfixAnnotations, empty_block_with_infix_annotations};
pub use context::*;

pub(crate) use dyst_ast::*;

#[cfg(test)]
pub(crate) mod tests;
