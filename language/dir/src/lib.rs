#![feature(default_field_values)]

extern crate self as destack_dir;

pub mod index;
mod schema;
mod source;
mod symbol;
mod table;
mod tree;
mod r#type;

pub use destack_dir_macros::TypeFold;
pub use index::*;
pub use schema::*;
pub use source::*;
pub use symbol::*;
pub use table::*;
pub use tree::*;
pub use r#type::*;
