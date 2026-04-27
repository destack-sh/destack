use std::path::Path;

use destack_core::stable_hash_key_value_128;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::id;
use crate::PackageId;

const MODULE_KEY_DOMAIN: &[u8] = b"module";

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

/// Stable key for one module within a package.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModuleKey(pub u128);

impl Serialize for ModuleKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        id::serialize_u128(self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for ModuleKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        id::deserialize_u128(deserializer).map(Self)
    }
}

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
        Self {
            package_id: package,
            module_key: ModuleKey::new(stable_hash_key_value_128(
                MODULE_KEY_DOMAIN,
                relative_path.to_string_lossy().as_bytes(),
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

        // include loader in hash if provided (for non-default loaders)
        let hash_input = match loader_salt {
            Some(salt) => format!("{}::{}", relative.to_string_lossy(), salt),
            None => relative.to_string_lossy().into_owned(),
        };

        Self {
            package_id: package,
            module_key: ModuleKey::new(stable_hash_key_value_128(
                MODULE_KEY_DOMAIN,
                hash_input.as_bytes(),
            )),
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
