use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;

/// Warnings during the check phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Check)]
pub enum CheckWarning {
    // -------------------------------------------------------------------------
    // 9xx: Unsupported / internal
    // -------------------------------------------------------------------------
    /// Unsupported node.
    #[diagnostic(code = "WC900", message = "unsupported construct")]
    UnsupportedConstruct { anchor: DiagnosticAnchor },
}
