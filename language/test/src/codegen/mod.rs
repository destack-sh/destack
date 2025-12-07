//! Codegen tests.
//!
//! Tests for verifying code generation output matches expected snapshots.
//! Each test is a directory containing:
//! - `dsconfig.json` with target definitions
//! - `src/` with source files
//! - `dist/<target>/` with expected output for each target
//!
//! The test runner compiles source files, emits to `dist-actual/`, and compares
//! against the expected `dist/` directory.

mod assert;
mod discover;
mod runner;

pub use runner::run_codegen_tests;
