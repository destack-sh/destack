use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;

/// Errors during the verify phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Verify)]
pub enum VerifyError {
    // -------------------------------------------------------------------------
    // 9xx: Internal issues
    // -------------------------------------------------------------------------
    /// Internal verify failure.
    #[diagnostic(code = "EV900", message = "{message}")]
    Internal {
        anchor: DiagnosticAnchor,
        message: String,
    },
}
