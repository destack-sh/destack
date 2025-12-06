use std::path::Path;

use crate::{fnv1a_32, PackageId};

/// Unique identifier for Modules.
///
/// ModuleId is hierarchical: it includes the PackageId and a local identifier.
/// This makes it stable across compiler runs (enables efficient cross-package caching).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModuleId {
    /// The package this module belongs to.
    pub package: PackageId,
    /// Local identifier within the package (hash of relative path).
    pub local: u32,
}

impl std::fmt::Debug for ModuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{:08x}", self.package, self.local)
    }
}

impl std::fmt::Display for ModuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{:08x}", self.package, self.local)
    }
}

impl ModuleId {
    /// Well-known ID for ephemeral/virtual modules (REPL, root).
    pub const EPHEMERAL: Self = Self {
        package: PackageId::EPHEMERAL,
        local: 0,
    };

    /// Create a ModuleId from package and local id.
    pub fn new(package: PackageId, local: u32) -> Self {
        Self { package, local }
    }

    /// Create a ModuleId from a package and relative path within the package.
    pub fn from_relative_path(package: PackageId, relative_path: &Path) -> Self {
        Self {
            package,
            local: fnv1a_32(relative_path.to_string_lossy().as_bytes()),
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
