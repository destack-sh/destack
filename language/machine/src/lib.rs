#![feature(default_field_values)]
#![feature(if_let_guard)]
#![feature(str_as_str)]

//! Destack Machine - MIR interpreter for compile-time evaluation.
//!
//! This crate provides an interpreter for Destack MIR, used for:
//! - Compile-time evaluation (comptime)
//! - REPL execution
//! - Debugging and stepping through code

pub mod diagnostic;
pub mod interpreter;
pub mod memory;

pub use diagnostic::*;
pub use interpreter::*;
pub use memory::*;
