use crate::DiagnosticAnchor;
use tspp_artifact_macros::Diagnostic;
use tspp_source::ModuleId;

/// Warnings during the emit phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Emit)]
pub enum EmitWarning {
    // -------------------------------------------------------------------------
    // types
    // -------------------------------------------------------------------------
    /// Imprecise type.
    #[diagnostic(id = "imprecise-type", message = "imprecise type")]
    ImpreciseType {
        anchor: DiagnosticAnchor,
        module: ModuleId,
    },

    // -------------------------------------------------------------------------
    // constructs
    // -------------------------------------------------------------------------
    /// Unexpected construct (recoverable).
    #[diagnostic(
        id = "recoverable-emission-construct",
        message = "unexpected construct"
    )]
    UnexpectedConstruct {
        anchor: DiagnosticAnchor,
        module: ModuleId,
    },
}
