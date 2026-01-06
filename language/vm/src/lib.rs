#![feature(default_field_values)]
#![feature(explicit_tail_calls)]
#![feature(if_let_guard)]
#![feature(str_as_str)]
#![allow(incomplete_features)]

pub mod diagnostic;
pub mod interpreter;
pub mod memory;

#[cfg(test)]
mod tests;

pub use diagnostic::*;
pub use interpreter::*;
pub use memory::*;
