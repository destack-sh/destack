use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use tspp_source::{ModuleId, PackageId, ProfileId};

use crate::{ArtifactDependency, ArtifactFingerprint, ArtifactKey, BuildId};

/// Reusable identity of one artifact result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
pub struct ArtifactVersion {
    /// The artifact key.
    pub key: ArtifactKey,
    /// The observed input fingerprint.
    pub fingerprint: ArtifactFingerprint,
}

impl Hash for ArtifactVersion {
    /// Hash by the fingerprint alone: it already covers the key.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.fingerprint.hash(state);
    }
}

impl ArtifactVersion {
    /// Create one artifact version from its declared dependency observations.
    pub fn new(
        key: ArtifactKey,
        build_id: BuildId,
        dependencies: impl IntoIterator<Item = ArtifactDependency>,
    ) -> Self {
        let fingerprint = ArtifactFingerprint::new(key, build_id, dependencies);

        Self { key, fingerprint }
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
