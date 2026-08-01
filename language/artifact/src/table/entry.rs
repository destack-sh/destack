use std::collections::BTreeMap;
use std::sync::Arc;

use destack_serde::Reflect;
use destack_source::Content;
use serde::{Deserialize, Serialize};

use crate::{
    ArtifactDependency, ArtifactFailure, ArtifactPayload, ArtifactVersion, DiagnosticRecord,
};

/// Dense in-process id for one artifact key.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArtifactId(pub(crate) u32);

impl ArtifactId {
    /// Return this dense artifact id as a table index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Compact in-process id for one immutable artifact binding.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArtifactBindingId(pub(crate) u32);

/// One reusable artifact result entry.
#[derive(Debug, Clone)]
pub(crate) struct ArtifactEntry {
    /// The exact terminal result.
    pub(crate) result: ArtifactResult,
    /// The diagnostics for this exact artifact version.
    pub(crate) diagnostics: Arc<[DiagnosticRecord]>,
    /// The sidecars for this exact artifact version.
    pub(crate) sidecars: Arc<[ArtifactSidecar]>,
}

/// One immutable artifact version and its exact dependency observations.
#[derive(Debug, Clone)]
pub struct ArtifactBinding {
    /// The reusable artifact result.
    pub version: ArtifactVersion,
    /// The exact dependency observations.
    pub dependencies: Arc<[ArtifactDependency]>,
}

impl ArtifactEntry {
    /// Create one successful artifact entry.
    pub(crate) fn ok(
        payload: ArtifactPayload,
        diagnostics: impl Into<Arc<[DiagnosticRecord]>>,
        sidecars: impl Into<Arc<[ArtifactSidecar]>>,
    ) -> Self {
        Self {
            result: ArtifactResult::Ok(payload),
            diagnostics: diagnostics.into(),
            sidecars: sidecars.into(),
        }
    }

    /// Create one failed artifact entry.
    pub(crate) fn failed(
        diagnostics: impl Into<Arc<[DiagnosticRecord]>>,
        sidecars: impl Into<Arc<[ArtifactSidecar]>>,
        failure: ArtifactFailure,
    ) -> Self {
        Self {
            result: ArtifactResult::Failed(failure),
            diagnostics: diagnostics.into(),
            sidecars: sidecars.into(),
        }
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

/// One named artifact sidecar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ArtifactSidecar {
    /// The sidecar name.
    pub name: String,
    /// The stable labels describing this sidecar.
    pub labels: BTreeMap<String, String>,
    /// The sidecar content.
    pub content: Content,
}

impl ArtifactSidecar {
    /// Create one artifact sidecar.
    pub fn new(
        name: impl Into<String>,
        labels: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
        content: Content,
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
