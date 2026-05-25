use std::fs;
use std::path::{Path, PathBuf};

use crate::core::{Case, fixtures_dir as test_fixtures_dir};

use super::{ConformanceCatalog, build_conformance_catalog_rows, render_conformance_catalog_table};

/// The runnable tests directory name inside one suite root.
pub const TESTS_DIRECTORY_NAME: &str = "tests";

/// The generated catalog report target.
pub const CATALOG_REPORT_TARGET: &str = "language/test/README.md";

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

/// Return one suite fixtures root.
pub fn suite_fixtures_dir(suite: &str) -> PathBuf {
    fixtures_dir().join(suite)
}

/// Return one runnable tests root under one conformance suite.
pub fn suite_tests_dir(suite: &str) -> PathBuf {
    suite_fixtures_dir(suite).join(TESTS_DIRECTORY_NAME)
}

/// Build one outer test case for one conformance suite.
pub fn suite_case(domain: &str, suite: &str) -> Case {
    let category = format!("destack_test::conformance::{domain}");
    let path = suite_fixtures_dir(suite);

    Case::directory(suite, path, category)
}

/// Update the generated catalog report target from the current suite metadata.
pub fn update_catalog_report_target() -> Result<Vec<PathBuf>, String> {
    // build the rendered catalog table once
    let catalog = ConformanceCatalog::load(&fixtures_dir())?;
    let rows = build_conformance_catalog_rows(&catalog);
    let table = render_conformance_catalog_table(&rows);

    // update the language testing document
    let target_path = repo_root_dir().join(CATALOG_REPORT_TARGET);
    let updated_files = if update_catalog_report_file(&target_path, &table)? {
        vec![target_path]
    } else {
        Vec::new()
    };

    Ok(updated_files)
}

/// Update one generated catalog section in one markdown target.
fn update_catalog_report_file(path: &Path, table: &str) -> Result<bool, String> {
    // read the current document
    let document = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;

    // splice in the rendered table
    let updated = super::replace_generated_section(&document, "conformance-catalog", table)?;

    // skip unchanged writes
    if updated == document {
        return Ok(false);
    }

    // persist the updated document
    fs::write(path, updated)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))?;

    Ok(true)
}
