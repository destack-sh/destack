use std::sync::Arc;

use std::collections::BTreeMap;

use destack_source::{DiagnosticCollection, FileContent};

use crate::{ArtifactDependency, ArtifactFailure};

/// One exact artifact version entry.
#[derive(Debug, Clone)]
pub(crate) struct ArtifactEntry {
    /// The exact terminal outcome.
    pub(crate) outcome: ArtifactOutcome,
    /// The exact dependencies.
    pub(crate) dependencies: Arc<[ArtifactDependency]>,
    /// The diagnostics for this exact artifact version.
    pub(crate) diagnostics: Arc<DiagnosticCollection>,
    /// The sidecars for this exact artifact version.
    pub(crate) sidecars: Arc<[ArtifactSidecar]>,
}

impl ArtifactEntry {
    /// Create one successful artifact entry.
    pub(crate) fn ok(
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        diagnostics: impl Into<Arc<DiagnosticCollection>>,
        sidecars: impl Into<Arc<[ArtifactSidecar]>>,
    ) -> Self {
        Self {
            outcome: ArtifactOutcome::Ok,
            dependencies: dependencies.into(),
            diagnostics: diagnostics.into(),
            sidecars: sidecars.into(),
        }
    }

    /// Create one failed artifact entry.
    pub(crate) fn failed(
        dependencies: impl Into<Arc<[ArtifactDependency]>>,
        diagnostics: impl Into<Arc<DiagnosticCollection>>,
        sidecars: impl Into<Arc<[ArtifactSidecar]>>,
        failure: ArtifactFailure,
    ) -> Self {
        Self {
            outcome: ArtifactOutcome::Failed(failure),
            dependencies: dependencies.into(),
            diagnostics: diagnostics.into(),
            sidecars: sidecars.into(),
        }
    }
}

/// One named artifact sidecar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactSidecar {
    /// The sidecar name.
    pub name: String,
    /// The stable labels describing this sidecar.
    pub labels: BTreeMap<String, String>,
    /// The sidecar content.
    pub content: FileContent,
}

impl ArtifactSidecar {
    /// Create one artifact sidecar.
    pub fn new(
        name: impl Into<String>,
        labels: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
        content: FileContent,
    ) -> Self {
        let labels = labels
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect();

        Self {
            name: name.into(),
            labels,
            content,
        }
    }

    /// Return whether this sidecar matches one name and label set.
    pub fn matches(&self, name: &str, labels: &BTreeMap<String, String>) -> bool {
        self.name == name && &self.labels == labels
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
