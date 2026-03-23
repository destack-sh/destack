use std::path::PathBuf;

use crate::{
    ArtifactRequirementError, ArtifactRequirementSet, DiagnosticAnchor, DiagnosticDefinition,
    TaskError,
};
use destack_compiler_macros::DefineError;
use destack_source::{FileType, PackageId, Uri};
use destack_workspace::{Program, TargetId};

/// Errors during the emit phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Emit)]
pub enum EmitError {
    // -------------------------------------------------------------------------
    // 0xx: Yield / requirement
    // -------------------------------------------------------------------------
    /// Wait for artifact requirement.
    #[error(code = "EW000", r#yield)]
    Yield { requirement: ArtifactRequirementSet },

    /// Yield requirement has failed.
    #[error(code = "EW001", yield_failed)]
    UnsatisfiedRequirement { requirement: ArtifactRequirementSet },

    /// Task was skipped due to stale versions.
    #[error(code = "EW002", message = "task skipped")]
    Skipped,

    // -------------------------------------------------------------------------
    // 1xx: Target issues
    // -------------------------------------------------------------------------
    /// Target not found in package.
    #[error(code = "EW100", message = "target not found: {target}")]
    TargetNotFound {
        package: PackageId,
        target: TargetId,
    },

    /// Emit is disabled by configuration.
    #[error(code = "EW101", message = "emit disabled by configuration (noEmit)")]
    NoEmit {
        package: PackageId,
        target: TargetId,
    },

    // -------------------------------------------------------------------------
    // 2xx: Output issues
    // -------------------------------------------------------------------------
    /// Output has invalid or missing output path.
    #[error(code = "EW200", message = "output has invalid output path: {uri}")]
    InvalidOutputPath { uri: Uri },

    /// Unsupported output.
    #[error(code = "EW201", message = "unsupported output type '{file_type}'")]
    UnsupportedOutput { uri: Uri, file_type: FileType },

    // -------------------------------------------------------------------------
    // 3xx: Write issues
    // -------------------------------------------------------------------------
    /// Failed to write output file.
    #[error(code = "EW300", message = "failed to write file '{path}': {message}")]
    FailedWrite {
        path: PathBuf,
        message: Option<String>,
    },

    // -------------------------------------------------------------------------
    // 9xx: Internal issues
    // -------------------------------------------------------------------------
    /// Internal emit failure.
    #[error(code = "EW900", message = "{message}")]
    Internal { message: String },
}
