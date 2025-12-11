//! Ecosystem tests: validate toolchain against real-world packages.

mod manifest;
pub mod runner;

pub use manifest::{EcosystemManifest, Tier};
pub use runner::run_ecosystem_tests;
