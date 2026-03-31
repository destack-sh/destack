#![feature(default_field_values)]
#![feature(if_let_guard)]

mod backend;
mod diagnostic;
mod dumper;
mod lower;

pub use backend::*;
pub use destack_js::*;
pub use diagnostic::*;
pub use dumper::*;
pub use lower::*;

#[cfg(test)]
pub(crate) mod tests;
