use crate::DiagnosticAnchor;
use tspp_artifact_macros::Diagnostic;

/// Warnings during the materialize phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Materialize)]
pub enum MaterializeWarning {
    // -------------------------------------------------------------------------
    // unsupported and internal failures
    // -------------------------------------------------------------------------
    /// Unsupported construct.
    #[diagnostic(id = "ignored-const-construct", message = "unsupported construct")]
    UnsupportedConstruct { anchor: DiagnosticAnchor },
}
