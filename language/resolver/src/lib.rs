#![feature(default_field_values)]
#![feature(if_let_guard)]
#![feature(once_cell_try)]

mod config;
mod package;
mod path;
mod resolve;
mod specifier;

pub(crate) use path::{CompiledAliasTable, ResolveRequest, ResolveRequestKind};
pub(crate) use resolve::ResolveFrame;

pub use path::Resolution;
pub use resolve::*;
pub use specifier::*;

#[cfg(test)]
mod tests;
