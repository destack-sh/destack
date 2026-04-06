#![feature(default_field_values)]
#![feature(if_let_guard)]

#[allow(unused_extern_crates)]
extern crate self as destack_source;

mod diagnostic;
mod edit;
mod file;
mod image;
mod tree;

pub use destack_core::StringId;
pub use destack_source_macros::AdaptImage;
pub use diagnostic::*;
pub use edit::*;
pub use file::*;
pub use image::*;
pub use tree::*;
