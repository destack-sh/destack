use std::sync::Arc;

use destack_source::DiagnosticSeverity;

use crate::{
    ArtifactDependency, ArtifactFailure, ArtifactPayload, ArtifactVersion, DiagnosticRecord,
};

/// One reusable artifact result entry.
#[derive(Debug)]
pub struct ArtifactEntry {
    /// The exact artifact version.
    pub version: ArtifactVersion,
    /// The exact dependencies observed while building this version.
    pub dependencies: Arc<[ArtifactDependency]>,
    /// The exact terminal result.
    pub(crate) result: ArtifactResult,
    /// The diagnostics for this exact artifact version.
    pub(crate) diagnostics: Arc<[DiagnosticRecord]>,
    /// Whether any diagnostic for this version carries error severity.
    pub(crate) has_errors: bool,
}

impl ArtifactEntry {
    /// Create one successful artifact entry.
    pub(crate) fn ok(
        version: ArtifactVersion,
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        payload: ArtifactPayload,
        diagnostics: impl Into<Arc<[DiagnosticRecord]>>,
    ) -> Self {
        let diagnostics = diagnostics.into();

        Self {
            version,
            dependencies: dependencies.into(),
            result: ArtifactResult::Ok(payload),
            has_errors: diagnostics
                .iter()
                .any(|record| record.diagnostic.severity == DiagnosticSeverity::Error),
            diagnostics,
        }
    }

    /// Create one failed artifact entry.
    pub(crate) fn failed(
        version: ArtifactVersion,
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        diagnostics: impl Into<Arc<[DiagnosticRecord]>>,
        failure: ArtifactFailure,
    ) -> Self {
        let diagnostics = diagnostics.into();

        Self {
            version,
            dependencies: dependencies.into(),
            result: ArtifactResult::Failed(failure),
            has_errors: diagnostics
                .iter()
                .any(|record| record.diagnostic.severity == DiagnosticSeverity::Error),
            diagnostics,
        }
    }

    /// Return the exact terminal outcome.
    pub fn outcome(&self) -> ArtifactOutcome {
        self.result.outcome()
    }

    /// Return the successful payload when present.
    pub fn payload(&self) -> Option<ArtifactPayload> {
        self.result.payload()
    }

    /// Return the diagnostics for this artifact version.
    pub fn diagnostics(&self) -> Arc<[DiagnosticRecord]> {
        self.diagnostics.clone()
    }

    /// Return whether this artifact version reported an error diagnostic.
    pub fn has_errors(&self) -> bool {
        self.has_errors
    }
}

/// Exact terminal result for one artifact version entry.
#[derive(Debug, Clone)]
pub(crate) enum ArtifactResult {
    /// The exact payload was produced.
    Ok(ArtifactPayload),
    /// The exact artifact attempt failed without a payload.
    Failed(ArtifactFailure),
}

impl ArtifactResult {
    /// Return the scalar terminal outcome.
    pub(crate) fn outcome(&self) -> ArtifactOutcome {
        match self {
            Self::Ok(_payload) => ArtifactOutcome::Ok,
            Self::Failed(failure) => ArtifactOutcome::Failed(failure.clone()),
        }
    }

    /// Return the successful payload when present.
    pub(crate) fn payload(&self) -> Option<ArtifactPayload> {
        match self {
            Self::Ok(payload) => Some(payload.clone()),
            Self::Failed(_failure) => None,
        }
    }
}

/// Exact terminal outcome for one artifact version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactOutcome {
    /// The exact payload was produced.
    Ok,
    /// The exact artifact attempt failed without a payload.
    Failed(ArtifactFailure),
}
