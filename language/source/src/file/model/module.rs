use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{PackageId, fnv1a_32};

/// Version of a module's compiled state (increments on recompilation).
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
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

/// Unique identifier for Modules.
///
/// ModuleId is hierarchical: it includes the PackageId and a local identifier.
/// This makes ModuleIds stable across compiler runs (for better cross-package caching).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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
    /// Well-known ID for ephemeral/virtual modules (e.g., REPL, root).
    pub const EPHEMERAL: Self = Self {
        package_id: PackageId::EPHEMERAL,
        local_id: 0,
    };

    /// Create a ModuleId from package and local id.
    pub fn new(package: PackageId, local: u32) -> Self {
        Self {
            package_id: package,
            local_id: local,
        }
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
        Self::from_path_with_loader(package, path, package_root, None)
    }

    /// Create a ModuleId from a package, path, and optional loader salt.
    ///
    /// When a non-default loader is used (e.g., `with { type: "text" }`), the loader
    /// name is included in the hash to ensure different loaders produce different ModuleIds.
    /// This allows the same file to be imported with different loaders as separate modules.
    pub fn from_path_with_loader(
        package: PackageId,
        path: &Path,
        package_root: Option<&Path>,
        loader_salt: Option<&str>,
    ) -> Self {
        let relative = if let Some(root) = package_root {
            path.strip_prefix(root).unwrap_or(path)
        } else {
            path
        };

        // include loader in hash if provided (for non-default loaders)
        let hash_input = match loader_salt {
            Some(salt) => format!("{}::{}", relative.to_string_lossy(), salt),
            None => relative.to_string_lossy().into_owned(),
        };

        Self {
            package_id: package,
            local_id: fnv1a_32(hash_input.as_bytes()),
        }
    }
}

/// A module id and version captured together.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModuleStamp {
    /// The module id.
    pub id: ModuleId,
    /// The module version.
    pub version: ModuleVersion,
}

impl std::fmt::Debug for ModuleStamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{id}@{version}", id = self.id, version = self.version)
    }
}

impl std::fmt::Display for ModuleStamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{id}@{version}", id = self.id, version = self.version)
    }
}

impl ModuleStamp {
    /// Create a new ModuleStamp.
    pub fn new(id: ModuleId, version: ModuleVersion) -> Self {
        Self { id, version }
    }
}
