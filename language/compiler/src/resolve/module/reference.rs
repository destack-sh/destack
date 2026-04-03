use std::path::{Path, PathBuf};

/// Resolve one source local or root-relative reference without filesystem canonicalization.
pub(crate) fn resolve_source_reference_path(
    base_path: &Path,
    specifier_path: &str,
    package_dir: &Path,
    root_dir: Option<&Path>,
) -> PathBuf {
    let base_directory = if specifier_path.starts_with('/') {
        resolved_reference_root(package_dir, root_dir)
    } else {
        base_path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .to_path_buf()
    };
    let relative_path = specifier_path.trim_start_matches('/');

    normalize_reference_path(&base_directory, Path::new(relative_path))
}

/// Resolve one configured source root for source root-relative references.
pub(crate) fn resolved_reference_root(package_dir: &Path, root_dir: Option<&Path>) -> PathBuf {
    match root_dir {
        Some(root_dir) if root_dir.is_absolute() => root_dir.to_path_buf(),
        Some(root_dir) => package_dir.join(root_dir),
        None => package_dir.to_path_buf(),
    }
}

/// Normalize one source reference path without filesystem canonicalization.
fn normalize_reference_path(base_directory: &Path, relative_path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in base_directory.join(relative_path).components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }

    normalized
}
