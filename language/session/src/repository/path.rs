use std::path::{Path, PathBuf};

/// Return the canonical path when available, otherwise the original path.
pub fn canonical_path_or_original(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
