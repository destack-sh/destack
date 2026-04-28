use std::sync::Arc;

use destack_source::DiagnosticCollection;

use crate::{ArtifactInput, ArtifactKey, ArtifactVersion};

/// Exact availability status for one artifact version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactStatus {
    /// The exact payload is published.
    Ready,
    /// The exact version is missing.
    Missing,
}

/// One exact artifact version entry.
#[derive(Debug, Clone)]
pub(crate) struct ArtifactEntry {
    /// The exact non-artifact inputs.
    pub(crate) inputs: Arc<[ArtifactInput]>,
    /// The exact artifact dependencies.
    pub(crate) dependencies: Arc<[ArtifactVersion]>,
    /// The diagnostics for this exact artifact version.
    pub(crate) diagnostics: Arc<DiagnosticCollection>,
}

impl ArtifactEntry {
    /// Create one published artifact entry.
    pub(crate) fn new(
        inputs: impl Into<Arc<[ArtifactInput]>>,
        dependencies: impl Into<Arc<[ArtifactVersion]>>,
        diagnostics: impl Into<Arc<DiagnosticCollection>>,
    ) -> Self {
        Self {
            inputs: inputs.into(),
            dependencies: dependencies.into(),
            diagnostics: diagnostics.into(),
        }
    }
}

/// One complete artifact publication record.
#[derive(Debug, Clone)]
pub struct ArtifactRecord<T> {
    /// The exact artifact version.
    pub version: ArtifactVersion,
    /// The typed artifact payload.
    pub payload: Arc<T>,
    /// The exact non-artifact inputs.
    pub inputs: Arc<[ArtifactInput]>,
    /// The exact artifact dependencies.
    pub dependencies: Arc<[ArtifactVersion]>,
    /// The diagnostics produced while building this artifact.
    pub diagnostics: Arc<DiagnosticCollection>,
}

impl<T> ArtifactRecord<T> {
    /// Create one complete artifact publication record.
    pub fn new(
        version: ArtifactVersion,
        payload: impl Into<Arc<T>>,
        inputs: impl Into<Arc<[ArtifactInput]>>,
        dependencies: impl Into<Arc<[ArtifactVersion]>>,
        diagnostics: impl Into<Arc<DiagnosticCollection>>,
    ) -> Self {
        Self {
            version,
            payload: payload.into(),
            inputs: inputs.into(),
            dependencies: dependencies.into(),
            diagnostics: diagnostics.into(),
        }
    }

    /// Return the artifact key for this record.
    pub fn key(&self) -> ArtifactKey {
        self.version.key
    }
}
