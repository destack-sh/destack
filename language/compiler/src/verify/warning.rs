use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;

/// Warnings during the verify phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Verify)]
pub enum VerifyWarning {
    // -------------------------------------------------------------------------
    // 9xx: Internal issues
    // -------------------------------------------------------------------------
    /// Internal verify warning.
    #[diagnostic(code = "WV900", message = "{message}")]
    Internal {
        anchor: DiagnosticAnchor,
        message: String,
    },
}
