use destack_artifact::{
    ArtifactKey, ArtifactSidecar, ArtifactVersion, DiagnosticContext, DiagnosticError,
    DiagnosticLike,
};
use destack_source::DiagnosticCollection;

use crate::{ArtifactAttemptRecorder, Moment, Revision};

/// Predecessor artifact binding selected for one provider attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtifactBase {
    /// The predecessor revision that owns the artifact binding.
    pub revision: Revision,
    /// The predecessor artifact version.
    pub version: ArtifactVersion,
}

impl ArtifactBase {
    /// Build one artifact base.
    pub const fn new(revision: Revision, version: ArtifactVersion) -> Self {
        Self { revision, version }
    }
}

/// Provider output sink for one artifact provider attempt.
pub trait ProviderContext: DiagnosticContext {
    /// Return the pinned repository revision for this attempt.
    fn revision(&self) -> Revision;

    /// Return the artifact key being built.
    fn artifact_key(&self) -> ArtifactKey;

    /// Return the predecessor artifact selected for this attempt.
    fn artifact_base(&self) -> Option<ArtifactBase> {
        None
    }

    /// Return whether this attempt should emit event traces.
    fn emit_events(&self) -> bool {
        false
    }

    /// Return the recorder for this artifact attempt, when the run is timed.
    fn recorder(&self) -> Option<&ArtifactAttemptRecorder> {
        None
    }

    /// Record one interior phase that started at one clock reading.
    fn emit_span(&self, name: &'static str, started: Moment) {
        if let Some(recorder) = self.recorder() {
            recorder.record_span(name, Some(started));
        }
    }

    /// Record one named counter for this attempt.
    fn emit_counter(&self, name: &'static str, value: u64) {
        if let Some(recorder) = self.recorder() {
            recorder.record_counter(name, value);
        }
    }

    /// Add an already-final diagnostic collection produced by this attempt.
    fn emit_diagnostics(&self, diagnostics: DiagnosticCollection);

    /// Add one sidecar produced by this attempt.
    fn emit_sidecar(&self, sidecar: ArtifactSidecar);

    /// Add one diagnostic produced by this attempt.
    fn emit(&self, diagnostic: &dyn DiagnosticLike) -> Result<(), DiagnosticError>;
}
