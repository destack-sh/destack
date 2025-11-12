pub mod compile;
mod diagnostic;
pub mod lex;
pub mod parse;
mod source;
pub mod transpile;
pub mod version;

pub(crate) use diagnostic::print_diagnostics;
pub(crate) use source::get_string_or_file;
