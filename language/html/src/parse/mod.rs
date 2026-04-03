mod body;
pub(crate) mod builder;
mod control;
mod doctype;
mod document;
mod foreign;
mod inline;
mod insert;
pub(crate) mod parser;
mod source;
mod table;
mod tag;
mod token;

pub use parser::*;
