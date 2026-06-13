use std::time::Instant;

use destack_artifact::{
    ArtifactKey, ArtifactSidecar, DiagnosticContext, DiagnosticError, DiagnosticLike,
};
use destack_source::DiagnosticCollection;

use crate::Revision;

use super::ArtifactTracer;

/// Provider output sink for one artifact provider attempt.
pub trait ProviderContext: DiagnosticContext {
    /// Return the pinned repository revision for this attempt.
    fn revision(&self) -> Revision;

    /// Return the artifact key being built.
    fn artifact_key(&self) -> ArtifactKey;

    /// Return whether this attempt should emit event traces.
    fn emit_events(&self) -> bool {
        false
    }

    /// Return the tracer recording this attempt, when the run is traced.
    fn tracer(&self) -> Option<&ArtifactTracer> {
        None
    }

    /// Record one interior phase that started at one instant.
    fn emit_span(&self, name: &'static str, started: Instant) {
        if let Some(tracer) = self.tracer() {
            tracer.record_span(name, started);
        }
    }

    /// Record one named counter for this attempt.
    fn emit_counter(&self, name: &'static str, value: u64) {
        if let Some(tracer) = self.tracer() {
            tracer.record_counter(name, value);
        }
    }

    /// Add an already-final diagnostic collection produced by this attempt.
    fn emit_diagnostics(&self, diagnostics: DiagnosticCollection);

    /// Add one sidecar produced by this attempt.
    fn emit_sidecar(&self, sidecar: ArtifactSidecar);

    /// Add one diagnostic produced by this attempt.
    fn emit(&self, diagnostic: &dyn DiagnosticLike) -> Result<(), DiagnosticError>;
}
