mod annotation;
mod argument;
mod block;
mod declaration;
mod dependency;
mod expression;
mod function;
mod key;
mod literal;
mod lowerer;
mod module;
mod operator;
mod path;
mod pattern;
mod property;
mod r#type;

pub use lowerer::*;
pub(in crate::generate::js) use module::*;
