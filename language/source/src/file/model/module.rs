use std::path::Path;

use serde::{Deserialize, Serialize};

use super::hash::{stable_source_id, stable_source_path};
use crate::PackageId;

const MODULE_KEY_DOMAIN: &[u8] = b"destack.source.module.v1";
const MODULE_LOADER_DEFAULT: &[u8] = b"default";

/// Stable key for one module within a package.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModuleKey(pub u128);

impl std::fmt::Debug for ModuleKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

impl std::fmt::Display for ModuleKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

impl ModuleKey {
    /// The ephemeral module key.
    pub const EPHEMERAL: Self = Self(0);

    /// Wrap a raw stable module key.
    pub const fn new(key: u128) -> Self {
        Self(key)
    }

    /// Return the raw stable key value.
    pub const fn raw(self) -> u128 {
        self.0
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
    /// The stable key for this module within its package.
    pub module_key: ModuleKey,
}

impl std::fmt::Debug for ModuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.package_id, self.module_key)
    }
}

impl std::fmt::Display for ModuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.package_id, self.module_key)
    }
}

impl ModuleId {
    /// Well-known ID for ephemeral/virtual modules (e.g., REPL, root).
    pub const EPHEMERAL: Self = Self {
        package_id: PackageId::EPHEMERAL,
        module_key: ModuleKey::EPHEMERAL,
    };

    /// Create a ModuleId from a package and module key.
    pub const fn new(package: PackageId, module_key: u128) -> Self {
        Self {
            package_id: package,
            module_key: ModuleKey::new(module_key),
        }
    }

    /// Create a ModuleId from a package and relative path within the package.
    pub fn from_relative_path(package: PackageId, relative_path: &Path) -> Self {
        let relative_path = stable_source_path(relative_path);

        Self {
            package_id: package,
            module_key: ModuleKey::new(stable_source_id(
                MODULE_KEY_DOMAIN,
                &[relative_path.as_bytes(), MODULE_LOADER_DEFAULT],
            )),
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

        let relative = stable_source_path(relative);
        let loader = loader_salt
            .map(str::as_bytes)
            .unwrap_or(MODULE_LOADER_DEFAULT);

        Self {
            package_id: package,
            module_key: ModuleKey::new(stable_source_id(
                MODULE_KEY_DOMAIN,
                &[relative.as_bytes(), loader],
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::ModuleId;
    use crate::PackageId;

    #[test]
    fn test_hash_module_loader_as_length_prefixed_component() {
        let package = PackageId::new(1);
        let left = ModuleId::from_path_with_loader(package, Path::new("a"), None, Some("b::c"));
        let right = ModuleId::from_path_with_loader(package, Path::new("a::b"), None, Some("c"));

        assert_ne!(left, right);
    }
}
