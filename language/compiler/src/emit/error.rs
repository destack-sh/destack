use std::path::PathBuf;

use crate::{
    DiagnosticAnchor, DiagnosticDefinition, TaskDependency, TaskDependencyError, TaskError,
};
use destack_compiler_macros::DefineError;
use destack_source::{FileType, PackageId, Uri};
use destack_workspace::{ArtifactId, Program};

/// Errors during the emit phase.
#[derive(Debug, Clone, PartialEq, DefineError)]
#[phase(Emit)]
pub enum EmitError {
    /// Wait for task dependency.
    #[error(code = "EX000", r#yield)]
    Yield { dependency: TaskDependency },

    /// Yield dependency has failed.
    #[error(code = "EX001", yield_failed)]
    UnsatisfiedDependency { dependency: TaskDependency },

    /// Target not found in package.
    #[error(code = "EX002", message = "target not found: {target}")]
    TargetNotFound { package: PackageId, target: String },

    /// Artifact has invalid or missing output path.
    #[error(code = "EX003", message = "artifact has invalid output path")]
    InvalidOutputPath { artifact: ArtifactId, uri: Uri },

    /// Unsupported artifact.
    #[error(code = "EX004", message = "unsupported artifact")]
    UnsupportedArtifact {
        artifact: ArtifactId,
        uri: Uri,
        file_type: FileType,
    },

    /// Failed to write output file.
    #[error(code = "EX005", message = "failed to write file")]
    FailedWrite {
        artifact: ArtifactId,
        path: PathBuf,
        message: Option<String>,
    },
}
