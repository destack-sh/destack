use std::path::Path;

use crate::{fnv1a_64, Uri};

/// Unique identifier for Packages.
///
/// PackageId is a stable hash based on the package's root path, making it
/// deterministic across compiler runs on the same machine.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PackageId(pub u64);

impl std::fmt::Debug for PackageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:016x}", self.0)
    }
}

impl std::fmt::Display for PackageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:016x}", self.0)
    }
}

impl PackageId {
    /// Well-known ID for ephemeral packages (e.g., REPL, root module).
    pub const EPHEMERAL: Self = Self(0);

    /// Create a PackageId from a raw hash value.
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Create a PackageId from a URI (deterministic).
    pub fn from_uri(uri: &Uri) -> Self {
        Self(fnv1a_64(uri.as_ref().as_bytes()))
    }

    /// Create a PackageId from a directory path (for physical packages).
    pub fn from_path(path: &Path) -> Self {
        let key = format!("physical:{}", path.to_string_lossy());
        Self(fnv1a_64(key.as_bytes()))
    }

    /// Create a PackageId for a synthetic package (loose files in a directory).
    pub fn from_synthetic_path(path: &Path) -> Self {
        let key = format!("synthetic:{}", path.to_string_lossy());
        Self(fnv1a_64(key.as_bytes()))
    }

    /// Get the raw id value.
    pub fn raw(&self) -> u64 {
        self.0
    }
}
