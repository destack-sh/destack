use std::sync::Arc;

use tspp_artifact::{
    Artifact, ArtifactDependency, ArtifactEntry, ArtifactKey, ArtifactPayload, ArtifactVersion,
    DiagnosticContext, DiagnosticError, DiagnosticLike, DiagnosticRecord,
};

use crate::{ArtifactAttemptRecorder, Moment, Revision, TraceEvent};

/// Retained artifact result selected as an incremental provider base.
#[derive(Debug)]
pub struct ArtifactBase {
    /// The retained base result.
    entry: Arc<ArtifactEntry>,
    /// The decoded base payload.
    payload: ArtifactPayload,
}

impl ArtifactBase {
    /// Build one retained artifact base.
    pub(crate) fn new(entry: Arc<ArtifactEntry>, payload: ArtifactPayload) -> Self {
        Self { entry, payload }
    }

    /// Return the base artifact version.
    pub fn version(&self) -> ArtifactVersion {
        self.entry.version
    }

    /// Return the base artifact's exact dependency observations.
    pub fn dependencies(&self) -> &[ArtifactDependency] {
        &self.entry.dependencies
    }

    /// Return the typed base artifact payload.
    pub fn artifact<A: Artifact>(&self) -> Option<Arc<A>> {
        A::from_payload(self.payload.clone())
    }
}

/// Provider output sink for one artifact provider attempt.
pub trait ProviderContext: DiagnosticContext {
    /// Return the pinned repository revision for this attempt.
    fn revision(&self) -> Revision;

    /// Return the artifact key being built.
    fn artifact_key(&self) -> ArtifactKey;

    /// Return the incremental artifact base selected for this attempt.
    fn artifact_base(&self) -> Option<&ArtifactBase> {
        None
    }

    /// Return the frozen dependency observations during provider execution.
    fn artifact_dependencies(&self) -> Option<&[ArtifactDependency]> {
        None
    }

    /// Return whether this attempt records detailed provider events.
    fn records_events(&self) -> bool {
        self.recorder()
            .is_some_and(ArtifactAttemptRecorder::records_events)
    }

    /// Return whether this attempt records timings and counters.
    fn records_timings(&self) -> bool {
        self.recorder()
            .is_some_and(ArtifactAttemptRecorder::records_timings)
    }

    /// Return the recorder for this artifact attempt, when the run is timed.
    fn recorder(&self) -> Option<&ArtifactAttemptRecorder> {
        None
    }

    /// Record one interior phase that started at one clock reading.
    fn record_span(&self, name: &'static str, started: Moment) {
        if let Some(recorder) = self.recorder() {
            recorder.record_span(name, Some(started));
        }
    }

    /// Record one named counter for this attempt.
    fn record_counter(&self, name: &'static str, value: u64) {
        if let Some(recorder) = self.recorder() {
            recorder.record_counter(name, value);
        }
    }

    /// Record several named counters for this attempt.
    fn record_counters(&self, counters: &[(&'static str, u64)]) {
        if let Some(recorder) = self.recorder() {
            recorder.record_counters(counters);
        }
    }

    /// Record ordered events for this attempt.
    fn record_events(&self, events: Vec<TraceEvent>) {
        if let Some(recorder) = self.recorder() {
            recorder.record_events(events);
        }
    }

    /// Record one dependency read during provider execution.
    fn observe(&self, _dependency: ArtifactDependency) {}

    /// Record one blocked artifact read during dependency collection.
    fn record_blocked(&self, _artifact_key: ArtifactKey) {}

    /// Add already-recorded diagnostics produced by this attempt.
    fn emit_diagnostics(&self, diagnostics: Vec<DiagnosticRecord>);

    /// Add one diagnostic produced by this attempt.
    fn emit(&self, diagnostic: &dyn DiagnosticLike) -> Result<(), DiagnosticError>;
}
