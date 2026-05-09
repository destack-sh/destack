use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_source::ModuleId;

/// Errors during the expand phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Expand)]
pub enum ExpandError {
    /// Invalid static if decorator.
    #[diagnostic(code = "EX100", message = "invalid static if: {message}")]
    InvalidStaticIf {
        anchor: DiagnosticAnchor,
        message: String,
    },

    /// Internal expand failure.
    #[diagnostic(code = "EX900", message = "internal error: {message}")]
    Internal {
        anchor: DiagnosticAnchor,
        module: ModuleId,
        message: String,
    },
}
