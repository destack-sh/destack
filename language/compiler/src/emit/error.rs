use std::path::PathBuf;

use crate::{
    DiagnosticAnchor, DiagnosticDefinition, TaskDependency, TaskDependencyError, TaskError,
};
use destack_compiler_macros::DefineError;
use destack_source::{FileType, PackageId, Uri};
use destack_workspace::{ArtifactId, Program, TargetId};

/// Errors during the emit phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Emit)]
pub enum EmitError {
    /// Wait for task dependency.
    #[error(code = "EW000", r#yield)]
    Yield { dependency: TaskDependency },

    /// Yield dependency has failed.
    #[error(code = "EW001", yield_failed)]
    UnsatisfiedDependency { dependency: TaskDependency },

    /// Target not found in package.
    #[error(code = "EW002", message = "target not found: {target}")]
    TargetNotFound {
        package: PackageId,
        target: TargetId,
    },

    /// Artifact has invalid or missing output path.
    #[error(code = "EW003", message = "artifact has invalid output path: {uri}")]
    InvalidOutputPath { artifact: ArtifactId, uri: Uri },

    /// Unsupported artifact.
    #[error(code = "EW004", message = "unsupported artifact type '{file_type}'")]
    UnsupportedArtifact {
        artifact: ArtifactId,
        uri: Uri,
        file_type: FileType,
    },

    /// Failed to write output file.
    #[error(code = "EW005", message = "failed to write file '{path}': {message}")]
    FailedWrite {
        artifact: ArtifactId,
        path: PathBuf,
        message: Option<String>,
    },

    /// Emit is disabled by configuration.
    #[error(code = "EW006", message = "emit disabled by configuration (noEmit)")]
    NoEmit {
        package: PackageId,
        target: TargetId,
    },
}
