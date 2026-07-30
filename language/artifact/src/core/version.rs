use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use destack_source::{ModuleId, PackageId, ProfileId};

use crate::{ArtifactDependency, ArtifactFingerprint, ArtifactKey};

/// Reusable identity of one artifact result.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct ArtifactVersion {
    /// The artifact key.
    pub key: ArtifactKey,
    /// The observed input fingerprint.
    pub fingerprint: ArtifactFingerprint,
}

impl ArtifactVersion {
    /// Create one artifact version from its declared dependency observations.
    pub fn new(
        key: ArtifactKey,
        build_fingerprint: &str,
        dependencies: impl IntoIterator<Item = ArtifactDependency>,
    ) -> Self {
        let fingerprint = ArtifactFingerprint::new(build_fingerprint, dependencies);

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
