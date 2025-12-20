#![feature(default_field_values)]
#![feature(if_let_guard)]
#![feature(str_as_str)]

pub mod diagnostic;
pub mod interpreter;
pub mod memory;

#[cfg(test)]
mod tests;

pub use diagnostic::*;
pub use interpreter::*;
pub use memory::*;
