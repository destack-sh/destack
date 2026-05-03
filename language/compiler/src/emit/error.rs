use std::path::PathBuf;

use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;
use destack_source::{FileType, TargetId, Uri};

/// Errors during the emit phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Error, phase = Emit)]
pub enum EmitError {
    // -------------------------------------------------------------------------
    // 1xx: Target issues
    // -------------------------------------------------------------------------
    /// Target not found in package.
    #[diagnostic(code = "EW100", message = "target not found: {target}")]
    TargetNotFound {
        anchor: DiagnosticAnchor,
        target: TargetId,
    },

    /// Emit is disabled by configuration.
    #[diagnostic(code = "EW101", message = "emit disabled by configuration (noEmit)")]
    NoEmit {
        anchor: DiagnosticAnchor,
        target: TargetId,
    },

    // -------------------------------------------------------------------------
    // 2xx: Output issues
    // -------------------------------------------------------------------------
    /// Output has invalid or missing output path.
    #[diagnostic(code = "EW200", message = "output has invalid output path: {uri}")]
    InvalidOutputPath { anchor: DiagnosticAnchor, uri: Uri },

    /// Unsupported output.
    #[diagnostic(code = "EW201", message = "unsupported output type '{file_type}'")]
    UnsupportedOutput {
        anchor: DiagnosticAnchor,
        uri: Uri,
        file_type: FileType,
    },

    // -------------------------------------------------------------------------
    // 3xx: Write issues
    // -------------------------------------------------------------------------
    /// Failed to write output file.
    #[diagnostic(code = "EW300", message = "failed to write file '{path}': {message}")]
    FailedWrite {
        anchor: DiagnosticAnchor,
        path: PathBuf,
        message: String,
    },

    // -------------------------------------------------------------------------
    // 9xx: Internal issues
    // -------------------------------------------------------------------------
    /// Internal emit failure.
    #[diagnostic(code = "EW900", message = "{message}")]
    Internal {
        anchor: DiagnosticAnchor,
        message: String,
    },
}
