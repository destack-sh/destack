use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::hash::stable_source_id;
use super::id;
use crate::PackageId;

const TARGET_KEY_DOMAIN: &[u8] = b"destack.source.target.v1";

/// Stable key for one target within a package.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TargetKey(pub u128);

impl Serialize for TargetKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        id::serialize_u128(self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for TargetKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        id::deserialize_u128(deserializer).map(Self)
    }
}

impl std::fmt::Display for TargetKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

impl TargetKey {
    /// Wrap a raw stable target key.
    pub const fn new(key: u128) -> Self {
        Self(key)
    }

    /// Return the raw stable key value.
    pub const fn raw(self) -> u128 {
        self.0
    }
}

/// Unique identifier for a build target within a package.
///
/// A target represents a build output with specific settings for code generation,
/// optimization, and output paths. Each target is associated with a profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TargetId {
    /// The owning package id.
    pub package_id: PackageId,
    /// The stable key for this target within its package.
    pub target_key: TargetKey,
}

impl std::fmt::Display for TargetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "t{package_id:032x}:{target_key}",
            package_id = self.package_id.raw(),
            target_key = self.target_key
        )
    }
}

impl TargetId {
    /// Create a new TargetId.
    pub fn new(package_id: PackageId, name: impl AsRef<str>) -> Self {
        Self {
            package_id,
            target_key: TargetKey::new(stable_source_id(
                TARGET_KEY_DOMAIN,
                &[name.as_ref().as_bytes()],
            )),
        }
    }

    /// Return the owning package id.
    pub fn package_id(&self) -> PackageId {
        self.package_id
    }
}
