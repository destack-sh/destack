use std::path::Path;

use crate::{Uri, fnv1a_32, fnv1a_64};

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

/// Unique identifier for Modules.
///
/// ModuleId is hierarchical: it includes the PackageId and a local identifier.
/// This makes ModuleIds stable across compiler runs (for better cross-package caching).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModuleId {
    /// The package this module belongs to.
    pub package_id: PackageId,
    /// Local identifier within the package (hash of relative path).
    pub local_id: u32,
}

impl std::fmt::Debug for ModuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{:08x}", self.package_id, self.local_id)
    }
}

impl std::fmt::Display for ModuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{:08x}", self.package_id, self.local_id)
    }
}

impl ModuleId {
    /// Well-known ID for ephemeral/virtual modules (REPL, root).
    pub const EPHEMERAL: Self = Self {
        package_id: PackageId::EPHEMERAL,
        local_id: 0,
    };

    /// Create a ModuleId from package and local id.
    pub fn new(package: PackageId, local: u32) -> Self {
        Self { package_id: package, local_id: local }
    }

    /// Create a ModuleId from a package and relative path within the package.
    pub fn from_relative_path(package: PackageId, relative_path: &Path) -> Self {
        Self {
            package_id: package,
            local_id: fnv1a_32(relative_path.to_string_lossy().as_bytes()),
        }
    }

    /// Create a ModuleId from a package and a path, computing the relative path.
    /// If the path is not within the package root, uses the full path as fallback.
    pub fn from_path(package: PackageId, path: &Path, package_root: Option<&Path>) -> Self {
        let relative = if let Some(root) = package_root {
            path.strip_prefix(root).unwrap_or(path)
        } else {
            path
        };
        Self::from_relative_path(package, relative)
    }
}

/// Version of a module's compiled state (increments on recompilation).
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct ModuleVersion(pub u64);

impl std::fmt::Debug for ModuleVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.0)
    }
}

impl std::fmt::Display for ModuleVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.0)
    }
}

impl ModuleVersion {
    /// Initial version.
    pub const INITIAL: Self = Self(0);

    /// Create a new ModuleVersion.
    pub fn new(version: u64) -> Self {
        Self(version)
    }

    /// Increment the version, returning the new value.
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}
