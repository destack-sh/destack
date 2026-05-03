use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;

/// Warnings during the elaborate phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Elaborate)]
pub enum ElaborateWarning {
    // -------------------------------------------------------------------------
    // 1xx: Configuration
    // -------------------------------------------------------------------------
    /// Implicit collection conversions are enabled with warnings.
    #[diagnostic(code = "WE100", message = "implicit collection conversion")]
    ImplicitCollectionConversion { anchor: DiagnosticAnchor },

    // -------------------------------------------------------------------------
    // 9xx: Unsupported / internal
    // -------------------------------------------------------------------------
    /// Unsupported node.
    #[diagnostic(code = "WE900", message = "unsupported construct")]
    UnsupportedConstruct { anchor: DiagnosticAnchor },
}
