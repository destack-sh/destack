use crate::DiagnosticAnchor;
use tspp_artifact_macros::Diagnostic;

/// Warnings during the expand phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Expand)]
pub enum ExpandWarning {
    /// Unsupported construct.
    #[diagnostic(
        id = "unsupported-expansion-construct",
        message = "unsupported construct"
    )]
    UnsupportedConstruct { anchor: DiagnosticAnchor },
}
