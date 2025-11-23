pub mod cli;
pub mod command;
pub mod console;

pub use command::{compile, lex, parse, resolve, transpile, version};
