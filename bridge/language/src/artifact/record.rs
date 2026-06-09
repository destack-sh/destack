use destack_artifact as artifact;
use destack_core as core;

use crate::{ArtifactDependency, ArtifactSidecar, ArtifactVersion, Diagnostic, bridge};

/// One interned string carried by an artifact record.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArtifactString {
    /// Canonical lowercase hex string id.
    pub id: String,
    /// Interned string text.
    pub text: String,
}

/// Self-contained raw artifact body crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRecord {
    /// The exact artifact version.
    pub version: ArtifactVersion,
    /// Serialized artifact image bytes.
    pub image: Vec<u8>,
    /// String pool needed to interpret interned ids in the payload.
    pub strings: Vec<ArtifactString>,
    /// Exact artifact dependencies.
    pub dependencies: Vec<ArtifactDependency>,
    /// Diagnostics recorded for this artifact version.
    pub diagnostics: Vec<Diagnostic>,
    /// Artifact sidecars recorded for this artifact version.
    pub sidecars: Vec<ArtifactSidecar>,
}

impl ArtifactString {
    /// Convert one core string entry into one bridge string entry.
    pub fn from_core(id: core::StringId, text: &str) -> Self {
        Self {
            id: format!("{:032x}", id.raw()),
            text: text.to_string(),
        }
    }
}

impl ArtifactRecord {
    /// Convert one artifact record into one bridge artifact record.
    pub fn from_artifact(record: artifact::ArtifactRecord) -> Self {
        let strings = record
            .strings
            .iter()
            .into_iter()
            .map(|(id, text)| ArtifactString::from_core(id, text))
            .collect();
        let dependencies = record
            .dependencies
            .into_iter()
            .map(ArtifactDependency::from_artifact)
            .collect();
        let diagnostics = record
            .diagnostics
            .to_vec()
            .into_iter()
            .map(Diagnostic::from_source)
            .collect();
        let sidecars = record
            .sidecars
            .into_iter()
            .map(ArtifactSidecar::from_artifact)
            .collect();

        Self {
            version: ArtifactVersion::from_artifact(record.version),
            image: record.image,
            strings,
            dependencies,
            diagnostics,
            sidecars,
        }
    }
}

impl From<artifact::ArtifactRecord> for ArtifactRecord {
    /// Convert one artifact record into one bridge artifact record.
    fn from(record: artifact::ArtifactRecord) -> Self {
        Self::from_artifact(record)
    }
}
