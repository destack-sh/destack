use std::fs;
use std::path::{Path, PathBuf};

/// Default extensions for Destack source files.
pub(super) const SOURCE_EXTENSIONS: &[&str] = &["ds"];

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
