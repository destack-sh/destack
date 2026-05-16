use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;

/// Warnings during the import phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Import)]
pub enum ImportWarning {
    /// Unused imports or unused re-exports.
    #[diagnostic(code = "WI100", message = "unused import")]
    UnusedImport { anchor: DiagnosticAnchor },
}
