use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;

/// Warnings during the analyze phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Analyze)]
pub enum AnalyzeWarning {
    // -------------------------------------------------------------------------
    // 9xx: Unsupported / internal
    // -------------------------------------------------------------------------
    /// Unsupported node.
    #[diagnostic(code = "WA900", message = "unsupported construct")]
    UnsupportedConstruct { anchor: DiagnosticAnchor },
}
