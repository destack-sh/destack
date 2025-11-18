#![feature(default_field_values)]
#![feature(if_let_guard)]

pub mod format;
pub use format::*;

#[cfg(test)]
mod tests;
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use tests::*;