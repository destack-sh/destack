mod body;
mod builder;
mod control;
mod doctype;
mod document;
mod foreign;
mod inline;
mod insert;
mod parser;
mod source;
mod table;
mod tag;
#[cfg(test)]
mod test;
mod token;

pub use parser::*;
