use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_source::ModuleId;

/// Errors during the expand phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Expand)]
pub enum ExpandError {
    /// Internal expand failure.
    #[diagnostic(code = "EX900", message = "internal error: {message}")]
    Internal {
        /// The source that triggered the internal failure.
        anchor: DiagnosticAnchor,
        /// The module being expanded.
        module: ModuleId,
        /// The internal failure message.
        message: String,
    },
}
