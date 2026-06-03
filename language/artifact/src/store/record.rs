use destack_core::StringPool;
use destack_source::DiagnosticCollection;
use serde::{Deserialize, Serialize};

use crate::{
    ArtifactDependency, ArtifactImage, ArtifactImageError, ArtifactPayload, ArtifactSidecar,
    ArtifactVersion,
};

/// Self-contained transport record for one exact artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRecord {
    /// The exact artifact version.
    pub version: ArtifactVersion,
    /// The serialized artifact image.
    pub image: Vec<u8>,
    /// String pool needed to interpret interned ids in the payload.
    pub strings: StringPool,
    /// The exact artifact dependencies.
    pub dependencies: Vec<ArtifactDependency>,
    /// Diagnostics recorded for this artifact version.
    pub diagnostics: DiagnosticCollection,
    /// Artifact sidecars recorded for this artifact version.
    pub sidecars: Vec<ArtifactSidecar>,
}

impl ArtifactRecord {
    /// Build one artifact record.
    pub fn new<T>(
        version: ArtifactVersion,
        payload: T,
        strings: StringPool,
        dependencies: Vec<ArtifactDependency>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
    ) -> Result<Self, ArtifactImageError>
    where
        T: Serialize,
    {
        let image = ArtifactImage::new(version, payload).serialize()?;

        Ok(Self {
            version,
            image,
            strings,
            dependencies,
            diagnostics,
            sidecars,
        })
    }

    /// Decode the serialized artifact image.
    pub fn artifact_image(&self) -> Result<ArtifactImage<ArtifactPayload>, ArtifactImageError> {
        ArtifactImage::deserialize(&self.image)
    }
}
