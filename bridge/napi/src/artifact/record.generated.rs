// generated bridge target, do not edit

use destack_bridge_language as bridge;

use napi_derive::napi;

use crate::{ArtifactDependency, ArtifactSidecar, ArtifactVersion, Diagnostic};

/// One interned string carried by an artifact record.
#[derive(Debug)]
#[napi(object)]
pub struct ArtifactString {
    /// Canonical lowercase hex string id.
    pub id: String,
    /// Interned string text.
    pub text: String,
}

impl ArtifactString {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::ArtifactString) -> Self {
        Self {
            id: value.id,
            text: value.text,
        }
    }
}

/// Self-contained raw artifact body crossing bridge boundaries.
#[derive(Debug)]
#[napi(object)]
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

impl ArtifactRecord {
    /// Convert one bridge value into one NAPI value.
    pub(crate) fn from_bridge(value: bridge::ArtifactRecord) -> Self {
        Self {
            version: ArtifactVersion::from_bridge(value.version),
            image: value.image,
            strings: value
                .strings
                .into_iter()
                .map(|item| ArtifactString::from_bridge(item))
                .collect(),
            dependencies: value
                .dependencies
                .into_iter()
                .map(|item| ArtifactDependency::from_bridge(item))
                .collect(),
            diagnostics: value
                .diagnostics
                .into_iter()
                .map(|item| Diagnostic::from_bridge(item))
                .collect(),
            sidecars: value
                .sidecars
                .into_iter()
                .map(|item| ArtifactSidecar::from_bridge(item))
                .collect(),
        }
    }
}
