use std::hash::{Hash, Hasher};

use rustc_hash::FxHasher;
use serde::{Deserialize, Serialize};

use crate::PackageId;

/// Unique identifier for a build target within a package.
///
/// A target represents a build output with specific settings for code generation,
/// optimization, and output paths. Each target is associated with a profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TargetId {
    /// The owning package id.
    pub package_id: PackageId,
    /// The stable hash of the target name within the package.
    pub target_name_hash: u64,
}

impl std::fmt::Display for TargetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "t{package_id:016x}:{target_name_hash:016x}",
            package_id = self.package_id.raw(),
            target_name_hash = self.target_name_hash
        )
    }
}

impl TargetId {
    /// Create a new TargetId.
    pub fn new(package_id: PackageId, name: impl AsRef<str>) -> Self {
        let mut hasher = FxHasher::default();
        name.as_ref().hash(&mut hasher);

        Self {
            package_id,
            target_name_hash: hasher.finish(),
        }
    }

    /// Return the owning package id.
    pub fn package_id(&self) -> PackageId {
        self.package_id
    }
}
