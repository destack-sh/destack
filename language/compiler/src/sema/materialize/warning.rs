use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;

/// Warnings during the materialize phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Materialize)]
pub enum MaterializeWarning {
    // -------------------------------------------------------------------------
    // unsupported and internal failures
    // -------------------------------------------------------------------------
    /// Unsupported construct.
    #[diagnostic(id = "ignored-comptime-construct", message = "unsupported construct")]
    UnsupportedConstruct { anchor: DiagnosticAnchor },
}
