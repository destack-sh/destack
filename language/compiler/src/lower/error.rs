use destack_artifact::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;

/// Errors during the lower phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Lower)]
pub enum LowerError {
    /// MIR lowering is unavailable.
    #[diagnostic(code = "EL900", message = "MIR lowering is unavailable")]
    Unavailable {
        /// Anchor the error to the affected module.
        anchor: DiagnosticAnchor,
    },
}
