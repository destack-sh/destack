use std::fs;
use std::path::{Path, PathBuf};

use crate::core::Case;

/// Default extensions for Destack source files.
pub(super) const SOURCE_EXTENSIONS: &[&str] = &[
    "ds", "d.ds", "ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs", "html", "css", "svg",
];

/// Discover emit cases recursively from one fixture root.
pub(super) fn discover_emit_cases(directory: &Path, category: &str) -> std::io::Result<Vec<Case>> {
    let mut cases = Vec::new();
    discover_emit_cases_recursive(directory, directory, category, false, &mut cases)?;
    cases.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(cases)
}

/// Discover source files in a directory recursively.
///
/// Returns all files matching the given extensions, sorted for consistent ordering.
pub(super) fn discover_source_files(
    dir: &Path,
    extensions: &[&str],
) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    discover_source_files_recursive(dir, extensions, &mut files)?;
    files.sort();
    Ok(files)
}

/// Discover source files in a directory recursively.
fn discover_source_files_recursive(
    dir: &Path,
    extensions: &[&str],
    files: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            discover_source_files_recursive(&path, extensions, files)?;
        } else if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy();
            if extensions.iter().any(|e| *e == ext_str) {
                files.push(path);
            }
        }
    }

    Ok(())
}

/// Discover emit cases recursively.
fn discover_emit_cases_recursive(
    directory: &Path,
    root: &Path,
    category: &str,
    is_skipped_parent: bool,
    cases: &mut Vec<Case>,
) -> std::io::Result<()> {
    if !directory.exists() {
        return Ok(());
    }

    let name = directory
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let is_hidden = name.starts_with('.');
    if is_hidden {
        return Ok(());
    }

    let is_skipped = is_skipped_parent || name.starts_with('_');
    let config_path = directory.join("destack.json");
    if config_path.is_file() {
        let relative = directory
            .strip_prefix(root)
            .unwrap_or(directory)
            .to_string_lossy()
            .replace('\\', "/");
        let case_name = if relative.is_empty() {
            directory
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_string()
        } else {
            relative
        };

        let case = Case::directory(case_name, directory.to_path_buf(), category.to_string())
            .with_skipped(is_skipped);
        cases.push(case);
        return Ok(());
    }

    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            discover_emit_cases_recursive(&path, root, category, is_skipped, cases)?;
        }
    }

    Ok(())
}

/// Collect all files in a directory recursively, returning paths relative to root.
pub(super) fn collect_files(dir: &Path, root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_files_recursive(dir, root, &mut files)?;
    files.sort();
    Ok(files)
}

/// Collect all files in a directory recursively.
fn collect_files_recursive(
    dir: &Path,
    root: &Path,
    files: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(&path, root, files)?;
        } else {
            let relative = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
            files.push(relative);
        }
    }

    Ok(())
}
