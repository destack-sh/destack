#![feature(default_field_values)]
#![feature(if_let_guard)]
#![allow(clippy::too_many_arguments)]

pub mod format;
pub use format::*;

#[cfg(test)]
mod tests;
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use tests::*;
