use destack_artifact as artifact;

use crate::{ArtifactVersion, ModuleId, ProfileId, bridge};

/// Typed projection of one resolved DIR artifact.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirResolved {
    /// Exact resolved artifact version.
    pub version: ArtifactVersion,
    /// Resolved module id.
    pub module: ModuleId,
    /// Resolved semantic profile.
    pub profile: ProfileId,
}

impl DirResolved {
    /// Project one resolved DIR artifact.
    pub fn from_artifact(
        version: artifact::ArtifactVersion,
        module: ModuleId,
        profile: ProfileId,
        _resolved: &artifact::DirResolved,
    ) -> Self {
        Self {
            version: ArtifactVersion::from_artifact(version),
            module,
            profile,
        }
    }
}
