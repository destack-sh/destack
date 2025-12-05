//! Smoke tests for parser and compiler.
//!
//! These tests verify that the parser and compiler don't crash on valid input
//! and produce no unexpected diagnostics.

mod compiler;
mod parser;

pub use compiler::run_compiler_smoke_tests;
pub use parser::run_parser_smoke_tests;
