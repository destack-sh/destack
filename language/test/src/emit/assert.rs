use std::collections::HashSet;
use std::fs;
use std::path::Path;

use destack_source::{DiffOptions, print_diff};

use crate::core::CaseResult;

use super::discover::collect_files;

/// Compare two directories recursively, failing if they differ.
pub(super) fn compare_directory(
    expected: &Path,
    actual: &Path,
    update_snapshots: bool,
) -> CaseResult {
    // snapshot refresh
    if update_snapshots {
        return update_snapshot_directory(expected, actual);
    }

    // expected directory
    let expected_files = match collect_files(expected, expected) {
        Ok(files) => files,
        Err(e) => {
            return CaseResult::Failed {
                message: format!("failed to read expected directory: {e}"),
            };
        }
    };
    let expected_set: HashSet<_> = expected_files.iter().collect();

    // actual directory
    let actual_files = match collect_files(actual, actual) {
        Ok(files) => files,
        Err(e) => {
            return CaseResult::Failed {
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
        CaseResult::Passed
    } else {
        CaseResult::Failed {
            message: errors.join("\n"),
        }
    }
}

/// Compare one exact text snapshot against one actual text payload.
pub(super) fn compare_text_snapshot(
    expected: &Path,
    actual: Option<&str>,
    update_snapshots: bool,
    noun: &str,
) -> CaseResult {
    // snapshot refresh
    if update_snapshots {
        return update_text_snapshot(expected, actual);
    }

    // exact expectation
    match (fs::read_to_string(expected), actual) {
        (Ok(expected_text), Some(actual_text)) => {
            if expected_text == actual_text {
                return CaseResult::Passed;
            }

            let options = DiffOptions::new().with_path(expected.display().to_string());
            print_diff(&expected_text, actual_text, &options);

            CaseResult::Failed {
                message: format!("{noun} mismatch: {}", expected.display()),
            }
        }
        (Ok(_), None) => CaseResult::Failed {
            message: format!("missing {noun}: {}", expected.display()),
        },
        (Err(_), Some(_)) => CaseResult::Failed {
            message: format!("unexpected {noun}: {}", expected.display()),
        },
        (Err(_), None) => CaseResult::Passed,
    }
}

/// Refresh one expected snapshot directory from one actual directory.
fn update_snapshot_directory(expected: &Path, actual: &Path) -> CaseResult {
    // require the actual directory
    if !actual.exists() {
        return CaseResult::Failed {
            message: format!("missing actual directory: {}", actual.display()),
        };
    }

    // replace the expected directory completely so deletions stay honest
    if expected.exists()
        && let Err(error) = fs::remove_dir_all(expected)
    {
        return CaseResult::Failed {
            message: format!(
                "failed to remove existing snapshot directory {}: {error}",
                expected.display()
            ),
        };
    }

    // rebuild the expected directory from the actual tree
    if let Err(error) = copy_directory(actual, expected) {
        return CaseResult::Failed {
            message: format!(
                "failed to refresh snapshot directory {} from {}: {error}",
                expected.display(),
                actual.display()
            ),
        };
    }

    CaseResult::Passed
}

/// Refresh one expected text snapshot from one actual text payload.
fn update_text_snapshot(expected: &Path, actual: Option<&str>) -> CaseResult {
    // remove snapshots when the actual text disappeared
    let Some(actual) = actual else {
        if expected.exists()
            && let Err(error) = fs::remove_file(expected)
        {
            return CaseResult::Failed {
                message: format!("failed to remove snapshot {}: {error}", expected.display()),
            };
        }

        return CaseResult::Passed;
    };

    // create the parent directory when needed
    if let Some(parent) = expected.parent()
        && let Err(error) = fs::create_dir_all(parent)
    {
        return CaseResult::Failed {
            message: format!(
                "failed to create snapshot directory {}: {error}",
                parent.display()
            ),
        };
    }

    // rewrite the snapshot exactly from the current output
    if let Err(error) = fs::write(expected, actual) {
        return CaseResult::Failed {
            message: format!("failed to write snapshot {}: {error}", expected.display()),
        };
    }

    CaseResult::Passed
}

/// Copy one directory tree recursively.
fn copy_directory(source: &Path, destination: &Path) -> std::io::Result<()> {
    // create the destination root
    fs::create_dir_all(destination)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());

        // recurse through subdirectories
        if source_path.is_dir() {
            copy_directory(&source_path, &destination_path)?;
        }
        // otherwise copy one file
        else {
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::copy(&source_path, &destination_path)?;
        }
    }

    Ok(())
}
