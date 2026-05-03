use std::path::PathBuf;

use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;

/// Warnings during the emit phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Emit)]
pub enum EmitWarning {
    // -------------------------------------------------------------------------
    // 1xx: File issues
    // -------------------------------------------------------------------------
    /// Overwriting an existing file.
    #[diagnostic(code = "WW100", message = "overwriting existing file")]
    OverwritingFile {
        anchor: DiagnosticAnchor,
        path: PathBuf,
    },

    /// Output file is unchanged from previous emit.
    #[diagnostic(code = "WW101", message = "file unchanged")]
    FileUnchanged {
        anchor: DiagnosticAnchor,
        path: PathBuf,
    },
}
