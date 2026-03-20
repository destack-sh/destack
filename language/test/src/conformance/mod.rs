mod catalog;
pub mod ecmascript;
mod expectation;
pub mod formatter;
pub mod node;
mod report;
pub mod web;

pub use catalog::*;
pub use expectation::*;
pub use report::*;

use std::path::PathBuf;

use crate::harness::fixtures_dir as test_fixtures_dir;

/// Return the conformance fixtures root.
pub fn fixtures_dir() -> PathBuf {
    test_fixtures_dir().join("conformance")
}

/// Return one conformance domain fixtures root.
pub fn domain_fixtures_dir(domain: &str) -> PathBuf {
    fixtures_dir().join(domain)
}
