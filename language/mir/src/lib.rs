#![feature(default_field_values)]
#![allow(hidden_glob_reexports)]

mod analyses;
mod build;
mod format;
pub mod parse;
mod rewrite;
mod schema;
pub mod source;
mod table;
mod tree;

pub use analyses::*;
pub use build::*;
pub use format::*;
pub use rewrite::*;
pub use schema::*;
pub use source::*;
pub use table::*;
pub use tree::*;

/// The module test trees belong to.
#[cfg(test)]
pub(crate) const TEST_MODULE: destack_source::ModuleId =
    destack_source::ModuleId::new(destack_source::PackageId::new(0), 0);
