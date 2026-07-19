use destack_artifact::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;

/// Errors during the lower phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Lower)]
pub enum LowerError {
    /// Construct is not supported by native compilation.
    #[diagnostic(
        id = "unsupported-native-construct",
        message = "native compilation does not support {construct}"
    )]
    Unsupported {
        /// Anchor the error to the unsupported construct.
        anchor: DiagnosticAnchor,
        /// The unsupported construct.
        construct: String,
    },
}
