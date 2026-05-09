use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;

/// Warnings during the expand phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Expand)]
pub enum ExpandWarning {
    /// Unsupported construct.
    #[diagnostic(code = "WX900", message = "unsupported construct")]
    UnsupportedConstruct { anchor: DiagnosticAnchor },
}
