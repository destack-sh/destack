use std::collections::HashSet;
use std::fs;
use std::path::Path;

use destack_source::{DiffOptions, print_diff};

use crate::harness::TestResult;

use super::discover::collect_files;

/// Compare two directories recursively, failing if they differ.
pub(super) fn compare_directory(expected: &Path, actual: &Path) -> TestResult {
    // expected directory
    let expected_files = match collect_files(expected, expected) {
        Ok(files) => files,
        Err(e) => {
            return TestResult::Failed {
                message: format!("failed to read expected directory: {e}"),
            };
        }
    };
    let expected_set: HashSet<_> = expected_files.iter().collect();

    // actual directory
    let actual_files = match collect_files(actual, actual) {
        Ok(files) => files,
        Err(e) => {
            return TestResult::Failed {
                message: format!("failed to read actual directory: {e}"),
            };
        }
    };
    let actual_set: HashSet<_> = actual_files.iter().collect();

    // collect errors
    let mut errors: Vec<String> = Vec::new();

    // check for missing files (in expected but not in actual)
    for path in &expected_files {
        if !actual_set.contains(path) {
            errors.push(format!("missing file: {}", path.display()));
        }
    }

    // check for extra files (in actual but not in expected)
    for path in &actual_files {
        if !expected_set.contains(path) {
            errors.push(format!("extra file: {}", path.display()));
        }
    }

    // compare content of common files
    for path in &expected_files {
        if actual_set.contains(path) {
            // read expected content
            let expected_path = expected.join(path);
            let expected_content = match fs::read_to_string(&expected_path) {
                Ok(c) => c,
                Err(e) => {
                    errors.push(format!("failed to read {}: {e}", expected_path.display()));
                    continue;
                }
            };

            // read actual content
            let actual_path = actual.join(path);
            let actual_content = match fs::read_to_string(&actual_path) {
                Ok(c) => c,
                Err(e) => {
                    errors.push(format!("failed to read {}: {e}", actual_path.display()));
                    continue;
                }
            };

            // compare file content
            if expected_content != actual_content {
                let options = DiffOptions::new().with_path(path.display().to_string());
                print_diff(&expected_content, &actual_content, &options);
                errors.push(format!("content mismatch: {}", path.display()));
            }
        }
    }

    if errors.is_empty() {
        TestResult::Passed
    } else {
        TestResult::Failed {
            message: errors.join("\n"),
        }
    }
}
