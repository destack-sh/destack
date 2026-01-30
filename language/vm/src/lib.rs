#![feature(default_field_values)]
#![feature(explicit_tail_calls)]
#![feature(if_let_guard)]
#![feature(str_as_str)]
#![allow(incomplete_features)]

pub mod diagnostic;
pub mod execute;
pub mod interpreter;
pub mod isolate;
pub mod memory;
pub mod options;
pub mod snapshot;
pub mod telemetry;

#[cfg(test)]
mod tests;

pub use diagnostic::*;
pub use interpreter::*;
pub use isolate::*;
pub use memory::*;
pub use options::*;
