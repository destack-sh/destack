#![allow(clippy::module_inception)]

mod artifact;
mod backend;
mod diagnostic;
mod lower;

pub use artifact::*;
pub use backend::*;
pub use diagnostic::*;

#[cfg(test)]
mod tests;
