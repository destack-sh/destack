//! Test harness utilities for running dynamic fixture-based tests.
//!
//! This module provides common infrastructure for discovering and running tests
//! from fixture directories, with proper CLI argument handling and output formatting.

mod diagnostic;
mod discover;
mod options;
mod print;
mod test;

pub use diagnostic::*;
pub use discover::*;
pub use options::*;
pub use print::*;
pub use test::*;

use std::path::PathBuf;

/// Get the fixtures directory path.
pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}
