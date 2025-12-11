use std::path::Path;
use std::{fs, io};

use super::TestCase;

/// Discover test files in a directory with given extensions.
/// Each file is treated as a test case.
/// Files prefixed with `_` are registered as skipped tests.
pub fn discover_test_files(
    directory: &Path,
    extensions: &[&str],
    category: &str,
) -> io::Result<Vec<TestCase>> {
    if !directory.exists() {
        return Ok(Vec::new());
    }

    let mut tests = Vec::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }

        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

        // check extension match
        let has_matching_extension = extensions.iter().any(|e| {
            if e.starts_with('.') {
                path.to_string_lossy().ends_with(e)
            } else {
                extension == *e
            }
        });
        if !has_matching_extension || name.is_empty() {
            continue;
        }

        // check if skipped (underscore prefix)
        let (test_name, skipped) = if let Some(stripped) = name.strip_prefix('_') {
            (stripped.to_string(), true)
        } else {
            (name.to_string(), false)
        };

        let case = TestCase::file(test_name, path.to_path_buf(), category).with_skipped(skipped);
        tests.push(case);
    }

    // sort by name for consistent ordering
    tests.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(tests)
}

/// Discover test directories in a parent directory.
/// Each subdirectory is treated as a test case.
/// Directories prefixed with `_` are registered as skipped tests.
/// Directories starting with `.` are ignored entirely.
pub fn discover_test_directories(directory: &Path, category: &str) -> io::Result<Vec<TestCase>> {
    if !directory.exists() {
        return Ok(Vec::new());
    }

    let mut tests = Vec::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if !path.is_dir() {
            continue;
        }

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() || name.starts_with('.') {
            continue;
        }

        // check if skipped (underscore prefix)
        let (test_name, is_skipped) = if let Some(stripped) = name.strip_prefix('_') {
            (stripped.to_string(), true)
        } else {
            (name, false)
        };

        let case = TestCase::directory(test_name, path, category).with_skipped(is_skipped);
        tests.push(case);
    }

    // sort by name for consistent ordering
    tests.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(tests)
}

/// Filter tests based on options.
pub fn filter_tests(tests: Vec<TestCase>, filter: Option<&str>) -> Vec<TestCase> {
    match filter {
        Some(f) => tests
            .into_iter()
            .filter(|t| {
                t.name.contains(f)
                    || t.full_name().contains(f)
                    || t.path.to_string_lossy().contains(f)
            })
            .collect(),
        None => tests,
    }
}
