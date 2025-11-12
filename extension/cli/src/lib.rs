pub mod command;
pub mod console;

pub use command::{compile, lex, parse, transpile, version};
pub use console::parse::{CommandApp, CommandArguments, CommandFn};
pub use console::table;
