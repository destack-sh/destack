use std::path::Path;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::hash::{stable_source_id, stable_source_path};
use super::id;
use crate::Uri;

const PACKAGE_ID_DOMAIN: &[u8] = b"destack.source.package.v1";
const PACKAGE_KIND_PHYSICAL: &[u8] = b"physical";
const PACKAGE_KIND_SYNTHETIC: &[u8] = b"synthetic";
const PACKAGE_KIND_URI: &[u8] = b"uri";

/// Unique identifier for one source package.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PackageId(pub u128);

impl Serialize for PackageId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        id::serialize_u128(self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for PackageId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        id::deserialize_u128(deserializer).map(Self)
    }
}

impl std::fmt::Debug for PackageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:032x}", self.0)
    }
}

impl std::fmt::Display for PackageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:032x}", self.0)
    }
}

impl PackageId {
    /// Well-known ID for ephemeral packages (e.g., REPL, root module).
    pub const EPHEMERAL: Self = Self(0);

    /// Create a PackageId from a raw hash value.
    pub const fn new(id: u128) -> Self {
        Self(id)
    }

    /// Create a PackageId from a URI (deterministic).
    pub fn from_uri(uri: &Uri) -> Self {
        Self(stable_source_id(
            PACKAGE_ID_DOMAIN,
            &[PACKAGE_KIND_URI, uri.as_ref().as_bytes()],
        ))
    }

    /// Create a PackageId from a directory path (for physical packages).
    pub fn from_path(path: &Path) -> Self {
        let path = stable_source_path(path);
        Self(stable_source_id(
            PACKAGE_ID_DOMAIN,
            &[PACKAGE_KIND_PHYSICAL, path.as_bytes()],
        ))
    }

    /// Create a PackageId for a synthetic package (loose files in a directory).
    pub fn from_synthetic_path(path: &Path) -> Self {
        let path = stable_source_path(path);
        Self(stable_source_id(
            PACKAGE_ID_DOMAIN,
            &[PACKAGE_KIND_SYNTHETIC, path.as_bytes()],
        ))
    }

    /// Get the raw id value.
    pub fn raw(&self) -> u128 {
        self.0
    }
}

/// Repository-local package snapshot version.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct PackageVersion(pub u128);

impl std::fmt::Debug for PackageVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.0)
    }
}

impl std::fmt::Display for PackageVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.0)
    }
}

impl PackageVersion {
    /// Initial version.
    pub const INITIAL: Self = Self(0);

    /// Create a new PackageVersion.
    pub fn new(version: u128) -> Self {
        Self(version)
    }

    /// Increment the version, returning the new value.
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// A package id and version captured together.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PackageStamp {
    /// The package id.
    pub id: PackageId,
    /// The package version.
    pub version: PackageVersion,
}

impl std::fmt::Debug for PackageStamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{id}@{version}", id = self.id, version = self.version)
    }
}

impl std::fmt::Display for PackageStamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{id}@{version}", id = self.id, version = self.version)
    }
}

impl PackageStamp {
    /// Create a new PackageStamp.
    pub fn new(id: PackageId, version: PackageVersion) -> Self {
        Self { id, version }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::PackageId;

    #[test]
    fn test_distinguish_physical_and_synthetic_packages() {
        let physical = PackageId::from_path(Path::new("workspace/app"));
        let synthetic = PackageId::from_synthetic_path(Path::new("workspace/app"));

        assert_ne!(physical, synthetic);
    }
}
