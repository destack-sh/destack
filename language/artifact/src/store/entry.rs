use std::sync::Arc;

use destack_source::DiagnosticCollection;

use crate::{ArtifactDependency, ArtifactFailure};

/// Exact terminal outcome for one artifact version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactOutcome {
    /// The exact payload is published.
    Ready,
    /// The exact artifact attempt produced diagnostics without a payload.
    Errored,
    /// The exact artifact attempt failed without a payload or user diagnostic result.
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
    /// Create one published artifact entry.
    pub(crate) fn ready(
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        diagnostics: impl Into<Arc<DiagnosticCollection>>,
    ) -> Self {
        Self {
            outcome: ArtifactOutcome::Ready,
            dependencies: dependencies.into(),
            diagnostics: diagnostics.into(),
        }
    }

    /// Create one errored artifact entry.
    pub(crate) fn errored(
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        diagnostics: impl Into<Arc<DiagnosticCollection>>,
    ) -> Self {
        Self {
            outcome: ArtifactOutcome::Errored,
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
