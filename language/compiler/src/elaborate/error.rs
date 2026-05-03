use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_source::ModuleId;

/// Errors during the elaborate phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Elaborate)]
pub enum ElaborateError {
    // -------------------------------------------------------------------------
    // 1xx: Configuration
    // -------------------------------------------------------------------------
    /// Implicit collection conversions are disabled by configuration.
    #[diagnostic(
        code = "EE100",
        message = "implicit collection conversions are disabled"
    )]
    ImplicitCollectionConversion { anchor: DiagnosticAnchor },

    // -------------------------------------------------------------------------
    // 9xx: Unsupported / internal
    // -------------------------------------------------------------------------
    /// Internal elaborate failure.
    #[diagnostic(code = "EE901", message = "internal error: {message}")]
    Internal {
        anchor: DiagnosticAnchor,
        module: ModuleId,
        message: String,
    },

    /// Unsupported node.
    #[diagnostic(code = "EE900", message = "unsupported construct")]
    UnsupportedConstruct { anchor: DiagnosticAnchor },
}
