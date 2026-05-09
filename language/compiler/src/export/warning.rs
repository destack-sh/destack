use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;

/// Warnings during the export phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Export)]
pub enum ExportWarning {
    /// Unused re-export.
    #[diagnostic(code = "WT100", message = "unused re-export")]
    UnusedReExport { anchor: DiagnosticAnchor },
}
