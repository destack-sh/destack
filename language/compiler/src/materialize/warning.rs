use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;

/// Warnings during the materialize phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Materialize)]
pub enum MaterializeWarning {
    // -------------------------------------------------------------------------
    // 9xx: Unsupported / internal
    // -------------------------------------------------------------------------
    /// Unsupported construct.
    #[diagnostic(code = "WM900", message = "unsupported construct")]
    UnsupportedConstruct { anchor: DiagnosticAnchor },
}
