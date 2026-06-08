use destack_artifact as artifact;

use crate::{FileContent, bridge};

/// One stable sidecar label crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArtifactSidecarLabel {
    /// Label key.
    pub key: String,
    /// Label value.
    pub value: String,
}

/// One named artifact sidecar crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactSidecar {
    /// Sidecar name.
    pub name: String,
    /// Stable labels describing this sidecar.
    pub labels: Vec<ArtifactSidecarLabel>,
    /// Sidecar content.
    pub content: FileContent,
}

impl ArtifactSidecar {
    /// Convert one artifact sidecar into one bridge sidecar.
    pub fn from_artifact(sidecar: artifact::ArtifactSidecar) -> Self {
        let labels = sidecar
            .labels
            .into_iter()
            .map(|(key, value)| ArtifactSidecarLabel { key, value })
            .collect();

        Self {
            name: sidecar.name,
            labels,
            content: sidecar.content.into(),
        }
    }
}

impl From<artifact::ArtifactSidecar> for ArtifactSidecar {
    /// Convert one artifact sidecar into one bridge sidecar.
    fn from(sidecar: artifact::ArtifactSidecar) -> Self {
        Self::from_artifact(sidecar)
    }
}
