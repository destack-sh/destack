use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_source::ModuleId;

/// Warnings during the generate phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Generate)]
pub enum GenerateWarning {
    // -------------------------------------------------------------------------
    // 1xx: Type warnings
    // -------------------------------------------------------------------------
    /// Imprecise type (loss of precision in codegen).
    #[diagnostic(code = "WG100", message = "imprecise type")]
    ImpreciseType {
        anchor: DiagnosticAnchor,
        module: ModuleId,
    },

    // -------------------------------------------------------------------------
    // 2xx: Construct warnings
    // -------------------------------------------------------------------------
    /// Unexpected construct (recoverable).
    #[diagnostic(code = "WG200", message = "unexpected construct")]
    UnexpectedConstruct {
        anchor: DiagnosticAnchor,
        module: ModuleId,
    },
}
