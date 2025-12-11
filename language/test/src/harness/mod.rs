mod context;
mod diagnostic;
pub mod diff;
mod discover;
mod expected_failures;
mod options;
pub mod print;
mod runner;
mod suite;
mod test;

pub use context::*;
pub use diagnostic::*;
pub use discover::*;
pub use expected_failures::*;
pub use options::*;
pub use print::*;
pub use runner::*;
pub use suite::*;
pub use test::*;

use std::path::PathBuf;

/// Get the fixtures directory path.
pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}
