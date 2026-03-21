mod catalog;
mod diff;
pub mod ecma;
pub mod formatter;
pub mod node;
mod report;
mod status;
pub mod web;

pub use catalog::*;
pub use diff::*;
pub use report::*;
pub use status::*;

use std::fs;
use std::path::{Path, PathBuf};

use crate::harness::fixtures_dir as test_fixtures_dir;

/// The runnable tests directory name inside one suite root.
pub const TESTS_DIRECTORY_NAME: &str = "tests";

/// The generated catalog report targets.
pub const CATALOG_REPORT_TARGETS: [&str; 2] = ["TESTING.md", "language/test/README.md"];

/// Return the conformance fixtures root.
pub fn fixtures_dir() -> PathBuf {
    test_fixtures_dir().join("conformance")
}

/// Return the repository root directory.
pub fn repo_root_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("language/test should live two levels below the repository root")
        .to_path_buf()
}

/// Return one conformance domain fixtures root.
pub fn domain_fixtures_dir(domain: &str) -> PathBuf {
    fixtures_dir().join(domain)
}

/// Return one suite fixtures root under one conformance domain.
pub fn suite_fixtures_dir(domain: &str, suite: &str) -> PathBuf {
    domain_fixtures_dir(domain).join(suite)
}

/// Return one runnable tests root under one conformance suite.
pub fn suite_tests_dir(domain: &str, suite: &str) -> PathBuf {
    suite_fixtures_dir(domain, suite).join(TESTS_DIRECTORY_NAME)
}

/// Update all generated catalog report targets from the current suite metadata.
pub fn update_catalog_report_targets() -> Result<Vec<PathBuf>, String> {
    let catalog = ConformanceCatalog::load(&fixtures_dir())?;
    let rows = build_conformance_catalog_rows(&catalog);
    let table = render_conformance_catalog_table(&rows);
    let mut updated_files = Vec::new();

    // update each generated markdown target
    for target in CATALOG_REPORT_TARGETS {
        let target_path = repo_root_dir().join(&target);
        if update_catalog_report_target(&target_path, &table)? {
            updated_files.push(target_path);
        }
    }

    Ok(updated_files)
}

/// Update one generated catalog section in one markdown target.
fn update_catalog_report_target(path: &Path, table: &str) -> Result<bool, String> {
    // read the current document
    let document = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;

    // splice in the rendered table
    let updated = replace_generated_section(&document, "conformance-catalog", table)?;

    // skip unchanged writes
    if updated == document {
        return Ok(false);
    }

    // persist the updated document
    fs::write(path, updated)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))?;

    Ok(true)
}
