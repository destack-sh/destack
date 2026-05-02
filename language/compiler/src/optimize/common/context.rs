use destack_artifact::DiagnosticBuilder;

use crate::{OptimizeError, OptimizeWarning};

/// Trait for emitting diagnostics (errors and warnings) during optimization.
///
/// This trait abstracts over the diagnostic emission interface, allowing
/// verification passes to work with `PipelineContext`.
pub trait DiagnosticEmitter {
    /// Emit an optimization error.
    fn emit_error(&self, error: impl Into<DiagnosticBuilder<OptimizeError>>);

    /// Emit an optimization warning.
    fn emit_warning(&self, warning: impl Into<DiagnosticBuilder<OptimizeWarning>>);

    /// Mark that aliasing violations were found (code is not strict-safe).
    fn mark_aliasing_violation(&self);
}
