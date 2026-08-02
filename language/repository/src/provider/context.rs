use std::sync::Arc;

use destack_artifact::{
    ArtifactDependency, ArtifactKey, ArtifactSidecar, ArtifactVersion, DiagnosticContext,
    DiagnosticError, DiagnosticLike, DiagnosticRecord,
};
use smallvec::SmallVec;

use crate::{ArtifactAttemptRecorder, Moment, Revision};

/// Retained predecessor artifact selected for one provider attempt.
#[derive(Debug)]
pub struct ArtifactBase {
    /// The predecessor artifact version.
    pub version: ArtifactVersion,
    /// The exact predecessor dependency observations.
    pub dependencies: Arc<[ArtifactDependency]>,
    /// Dependency ordinals that may differ in the requested revision.
    pub(crate) dirty_dependencies: SmallVec<[u32; 2]>,
    /// The retained predecessor artifact binding.
    _binding_pin: destack_artifact::ArtifactBindingPin,
}

impl ArtifactBase {
    /// Build one retained artifact base.
    pub(crate) fn new(
        version: ArtifactVersion,
        dependencies: Arc<[ArtifactDependency]>,
        dirty_dependencies: SmallVec<[u32; 2]>,
        binding_pin: destack_artifact::ArtifactBindingPin,
    ) -> Self {
        Self {
            version,
            dependencies,
            dirty_dependencies,
            _binding_pin: binding_pin,
        }
    }

    /// Return whether one predecessor dependency may differ in the requested revision.
    pub(crate) fn is_dependency_dirty(&self, dependency: usize) -> bool {
        self.dirty_dependencies
            .binary_search(&(dependency as u32))
            .is_ok()
    }
}

/// Provider output sink for one artifact provider attempt.
pub trait ProviderContext: DiagnosticContext {
    /// Return the pinned repository revision for this attempt.
    fn revision(&self) -> Revision;

    /// Return the artifact key being built.
    fn artifact_key(&self) -> ArtifactKey;

    /// Return the predecessor artifact selected for this attempt.
    fn artifact_base(&self) -> Option<&ArtifactBase> {
        None
    }

    /// Return the frozen dependency observations during provider execution.
    fn artifact_dependencies(&self) -> Option<&[ArtifactDependency]> {
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

    /// Record one dependency read during provider execution.
    fn observe(&self, _dependency: ArtifactDependency) {}

    /// Record one blocked artifact read during dependency collection.
    fn record_blocked(&self, _artifact_key: ArtifactKey) {}

    /// Add already-recorded diagnostics produced by this attempt.
    fn emit_diagnostics(&self, diagnostics: Vec<DiagnosticRecord>);

    /// Add one sidecar produced by this attempt.
    fn emit_sidecar(&self, sidecar: ArtifactSidecar);

    /// Add one diagnostic produced by this attempt.
    fn emit(&self, diagnostic: &dyn DiagnosticLike) -> Result<(), DiagnosticError>;
}
