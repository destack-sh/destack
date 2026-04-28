#![feature(default_field_values)]
#![feature(if_let_guard)]

#[allow(unused_extern_crates)]
extern crate self as destack_source;

mod diagnostic;
mod edit;
mod file;
mod tree;

pub use destack_core::StringId;
pub use diagnostic::*;
pub use edit::*;
pub use file::*;
pub use tree::*;
