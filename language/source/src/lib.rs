#![feature(default_field_values)]

#[allow(unused_extern_crates)]
extern crate self as destack_source;

mod diagnostic;
mod edit;
mod file;
mod provenance;
mod schema;
mod tree;

pub use destack_core::StringId;
pub use diagnostic::*;
pub use edit::*;
pub use file::*;
pub use provenance::*;
pub use schema::*;
pub use tree::*;
