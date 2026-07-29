use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use destack_source::{ModuleId, PackageId, ProfileId};

use crate::ArtifactKey;

/// Deterministic identity of one produced artifact result.
#[repr(transparent)]
#[derive(
    Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize, Reflect,
)]
pub struct ArtifactDigest(pub [u8; 32]);

impl std::fmt::Debug for ArtifactDigest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }

        Ok(())
    }
}

impl std::fmt::Display for ArtifactDigest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, formatter)
    }
}

/// Exact identity of one artifact result.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct ArtifactVersion {
    /// The artifact key.
    pub key: ArtifactKey,
    /// The exact produced result digest.
    pub digest: ArtifactDigest,
}

impl ArtifactVersion {
    /// Create one artifact version from its produced result digest.
    pub const fn new(key: ArtifactKey, digest: ArtifactDigest) -> Self {
        Self { key, digest }
    }

    /// Return the package referenced by this artifact version when one exists.
    pub fn package_id(&self) -> Option<PackageId> {
        self.key.package_id()
    }

    /// Return the module referenced by this artifact version when one exists.
    pub fn module_id(&self) -> Option<ModuleId> {
        self.key.module_id()
    }

    /// Return the profile referenced by this artifact version when one exists.
    pub fn profile_id(&self) -> Option<ProfileId> {
        self.key.profile_id()
    }
}
