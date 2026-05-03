use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;

/// Warnings during the execute phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Execute)]
pub enum ExecuteWarning {
    // -------------------------------------------------------------------------
    // 9xx: Unsupported / internal
    // -------------------------------------------------------------------------
    /// Unsupported construct.
    #[diagnostic(code = "WX900", message = "unsupported construct")]
    UnsupportedConstruct { anchor: DiagnosticAnchor },
}
