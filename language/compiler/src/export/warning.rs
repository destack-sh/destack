use crate::DiagnosticAnchor;
use tspp_artifact_macros::Diagnostic;

/// Warnings during the export phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Export)]
pub enum ExportWarning {
    /// Unused re-export.
    #[diagnostic(id = "unused-re-export", message = "unused re-export")]
    UnusedReExport { anchor: DiagnosticAnchor },
}
