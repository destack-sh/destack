#![feature(default_field_values)]
#![feature(if_let_guard)]

mod backend;
mod bundle;
mod diagnostic;
mod dumper;
mod emit;
mod format;
mod lower;
mod minify;
mod plan;
mod print;
mod tree;

pub use backend::*;
pub use bundle::*;
pub use diagnostic::*;
pub use dumper::*;
pub use emit::*;
pub use format::*;
pub use lower::*;
pub use minify::*;
pub use plan::*;
pub use print::*;
pub use tree::*;

#[cfg(test)]
pub(crate) mod tests;
