//! Markdown-driven tests.
//!
//! Tests defined in markdown files for type checking and diagnostics.
//!
//! ## Format
//!
//! Markdown test files follow the ezno-style format:
//!
//! ```markdown
//! ## Section Name
//!
//! ### Test Name
//!
//! ```ds
//! const x: string = 5
//! ```
//!
//! - Expected error message 1
//! - Expected error message 2
//! ```
//!
//! See `fixtures/mdtest/README.md` for full documentation.

mod parser;
mod runner;

pub use parser::{MdTestCase, parse_mdtest, parse_mdtest_file};
pub use runner::run_mdtests;
