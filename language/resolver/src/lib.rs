#![feature(default_field_values)]
#![feature(if_let_guard)]
#![feature(once_cell_try)]

mod resolve;

pub(crate) use resolve::{ResolvePath, ResolvePathKind};

pub use resolve::{Resolution, *};

#[cfg(test)]
mod tests;
