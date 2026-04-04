use std::hash::{Hash, Hasher};

use rustc_hash::FxHasher;
use serde::{Deserialize, Serialize};

use crate::PackageId;

/// Unique identifier for a build target within a package.
///
/// A target represents a build output with specific settings for code generation,
/// optimization, and output paths. Each target is associated with a profile.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TargetId(pub u64);

impl std::fmt::Display for TargetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "t{:016x}", self.0)
    }
}

impl TargetId {
    /// Create a new TargetId.
    pub fn new(package_id: PackageId, name: impl AsRef<str>) -> Self {
        let mut hasher = FxHasher::default();
        package_id.hash(&mut hasher);
        name.as_ref().hash(&mut hasher);

        Self(hasher.finish())
    }
}
