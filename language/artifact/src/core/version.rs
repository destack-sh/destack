use serde::{Deserialize, Serialize};

use destack_source::{ModuleId, PackageId, ProfileId};

use crate::{ArtifactDependency, ArtifactFingerprint, ArtifactKey};

/// One exact live artifact version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ArtifactVersion {
    /// The semantic artifact slot.
    pub key: ArtifactKey,
    /// The exact semantic fingerprint.
    pub fingerprint: ArtifactFingerprint,
}

impl ArtifactVersion {
    /// Create one artifact version from the complete recorded build dependencies.
    pub fn new(
        key: ArtifactKey,
        dependencies: impl IntoIterator<Item = ArtifactDependency>,
    ) -> Self {
        let fingerprint = ArtifactFingerprint::new(key, dependencies);

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
