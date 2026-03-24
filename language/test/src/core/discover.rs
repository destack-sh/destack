use std::path::{Path, PathBuf};
use std::{fs, io};

use super::Case;

/// Return the language test fixtures root.
pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

/// Return the repository root directory.
pub fn repo_root_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("language/test should live two levels below the repository root")
        .to_path_buf()
}

/// Return the shared materialized test output root.
pub fn test_output_dir() -> PathBuf {
    repo_root_dir().join("target").join("destack-test")
}

/// Discover file cases in one directory.
/// Each matching file becomes one case.
/// Files prefixed with `_` are registered as skipped cases.
pub fn discover_file_cases(
    directory: &Path,
    extensions: &[&str],
    category: &str,
) -> io::Result<Vec<Case>> {
    // treat missing directories as empty case sets
    if !directory.exists() {
        return Ok(Vec::new());
    }

    let mut cases = Vec::new();

    // inspect each direct child file
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

        // skipped prefix
        let (case_name, skipped) = if let Some(stripped) = name.strip_prefix('_') {
            (stripped.to_string(), true)
        } else {
            (name.to_string(), false)
        };

        let case = Case::file(case_name, path.to_path_buf(), category).with_skipped(skipped);
        cases.push(case);
    }

    // stable order
    cases.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(cases)
}

/// Discover directory cases in one parent directory.
/// Each subdirectory becomes one case.
/// Directories prefixed with `_` are registered as skipped cases.
/// Directories starting with `.` are ignored entirely.
pub fn discover_directory_cases(directory: &Path, category: &str) -> io::Result<Vec<Case>> {
    // treat missing directories as empty case sets
    if !directory.exists() {
        return Ok(Vec::new());
    }

    let mut cases = Vec::new();

    // inspect each direct child directory
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

        // skipped prefix
        let (case_name, is_skipped) = if let Some(stripped) = name.strip_prefix('_') {
            (stripped.to_string(), true)
        } else {
            (name, false)
        };

        let case = Case::directory(case_name, path, category).with_skipped(is_skipped);
        cases.push(case);
    }

    // stable order
    cases.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(cases)
}

/// Filter cases by the run filter when present.
pub fn filter_cases(cases: Vec<Case>, filter: Option<&str>) -> Vec<Case> {
    // keep the full case list when no filter was provided
    match filter {
        Some(f) => {
            let has_exact_match = cases.iter().any(|case| {
                case.name == f || case.full_name() == f || case.path.to_string_lossy() == f
            });

            cases
                .into_iter()
                .filter(|case| {
                    if has_exact_match {
                        case.name == f || case.full_name() == f || case.path.to_string_lossy() == f
                    } else {
                        case.name.contains(f)
                            || case.full_name().contains(f)
                            || case.path.to_string_lossy().contains(f)
                    }
                })
                .collect()
        }
        None => cases,
    }
}
