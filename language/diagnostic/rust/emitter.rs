use crate::Diagnostic;

/// Emitter of Diagnostics.
pub trait DiagnosticEmitter {
    /// Emit a diagnostic.
    fn emit(&self, diagnostic: Diagnostic);
}
