use std::sync::Arc;

use destack_source::DiagnosticCollection;

use crate::{ArtifactDependency, ArtifactFailure};

/// Exact terminal outcome for one artifact version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactOutcome {
    /// The exact payload was produced.
    Ok,
    /// The exact artifact attempt failed without a payload.
    Failed(ArtifactFailure),
}

/// One exact artifact version entry.
#[derive(Debug, Clone)]
pub(crate) struct ArtifactEntry {
    /// The exact terminal outcome.
    pub(crate) outcome: ArtifactOutcome,
    /// The exact dependencies.
    pub(crate) dependencies: Arc<[ArtifactDependency]>,
    /// The diagnostics for this exact artifact version.
    pub(crate) diagnostics: Arc<DiagnosticCollection>,
}

impl ArtifactEntry {
    /// Create one successful artifact entry.
    pub(crate) fn ok(
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        diagnostics: impl Into<Arc<DiagnosticCollection>>,
    ) -> Self {
        Self {
            outcome: ArtifactOutcome::Ok,
            dependencies: dependencies.into(),
            diagnostics: diagnostics.into(),
        }
    }

    /// Create one failed artifact entry.
    pub(crate) fn failed(
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        diagnostics: impl Into<Arc<DiagnosticCollection>>,
        failure: ArtifactFailure,
    ) -> Self {
        Self {
            outcome: ArtifactOutcome::Failed(failure),
            dependencies: dependencies.into(),
            diagnostics: diagnostics.into(),
        }
    }
}
