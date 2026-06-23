use crate::{ArtifactKey, bridge};

/// External artifact version crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArtifactVersion {
    /// Semantic artifact slot.
    pub key: ArtifactKey,
    /// Exact semantic fingerprint.
    pub fingerprint: String,
}

impl ArtifactVersion {
    /// Convert one artifact version into one bridge artifact version.
    pub fn from_artifact(version: destack_artifact::ArtifactVersion) -> Self {
        Self {
            key: ArtifactKey::from_artifact(version.key),
            fingerprint: version.fingerprint.to_string(),
        }
    }
}

impl From<destack_artifact::ArtifactVersion> for ArtifactVersion {
    /// Convert one artifact version into one bridge artifact version.
    fn from(version: destack_artifact::ArtifactVersion) -> Self {
        Self::from_artifact(version)
    }
}
