use crate::Diagnostic;

/// Handler of Diagnostics.
pub trait DiagnosticHandler {
    /// Handle a diagnostic.
    fn handle(&self, diagnostic: Diagnostic);
}
