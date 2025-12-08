use std::path::PathBuf;

use destack_source::{FileType, PackageId, Uri};

use crate::{DiagnosticAnchor, TaskDependency, TaskDependencyError, TaskError, TaskPhase};

use destack_workspace::{ArtifactId, Program};

/// Error when emitting output.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum EmitError {
    /// Wait for task dependency.
    Yield { dependency: TaskDependency },
    /// Yield dependency has failed.
    UnsatisfiedDependency { dependency: TaskDependency },
    /// Target not found in package.
    TargetNotFound { package: PackageId, target: String },
    /// Artifact has invalid or missing output path.
    InvalidOutputPath { artifact: ArtifactId, uri: Uri },
    /// Unsupported artifact.
    UnsupportedArtifact {
        artifact: ArtifactId,
        uri: Uri,
        file_type: FileType,
    },
    /// Failed to write output file.
    FailedWrite {
        artifact: ArtifactId,
        path: PathBuf,
        message: Option<String>,
    },
}

impl From<TaskDependencyError> for EmitError {
    fn from(e: TaskDependencyError) -> Self {
        match e {
            TaskDependencyError::NotReady { dependency } => Self::Yield { dependency },
            TaskDependencyError::Failed { dependency } => {
                Self::UnsatisfiedDependency { dependency }
            }
        }
    }
}

impl TryFrom<EmitError> for TaskDependency {
    type Error = EmitError;

    fn try_from(error: EmitError) -> Result<Self, Self::Error> {
        match error {
            EmitError::Yield { dependency } => Ok(dependency),
            _ => Err(error),
        }
    }
}

impl EmitError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Yield { .. } => 0,
            Self::UnsatisfiedDependency { .. } => 1,
            Self::TargetNotFound { .. } => 2,
            Self::InvalidOutputPath { .. } => 3,
            Self::UnsupportedArtifact { .. } => 4,
            Self::FailedWrite { .. } => 5,
        }
    }

    /// Get the anchor of the error.
    pub fn anchor(&self) -> DiagnosticAnchor {
        match self {
            Self::Yield { dependency } => dependency.anchor(),
            Self::UnsatisfiedDependency { dependency } => dependency.anchor(),
            Self::TargetNotFound { package, .. } => DiagnosticAnchor::Package(*package),
            Self::InvalidOutputPath { .. } => DiagnosticAnchor::Global,
            Self::UnsupportedArtifact { .. } => DiagnosticAnchor::Global,
            Self::FailedWrite { .. } => DiagnosticAnchor::Global,
        }
    }

    /// Get the message of the error.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::Yield { .. } => "pending dependency".to_string(),
            Self::UnsatisfiedDependency { .. } => "unsatisfied dependency".to_string(),
            Self::TargetNotFound { target, .. } => format!("target not found: {target}"),
            Self::InvalidOutputPath { uri, .. } => {
                format!("artifact has invalid output path: {uri}")
            }
            Self::UnsupportedArtifact { uri, file_type, .. } => {
                format!("unsupported artifact: {uri} (file type: {file_type:?})")
            }
            Self::FailedWrite { path, message, .. } => {
                let base = format!("failed to write file: {}", path.display());
                if let Some(msg) = message {
                    format!("{base}: {msg}")
                } else {
                    base
                }
            }
        }
    }
}

impl std::fmt::Display for EmitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmitError")
            .field(
                "code",
                &format!("E{}{:03}", TaskPhase::Emit.letter(), self.sub_code()),
            )
            .finish()
    }
}

pub type EmitResult<T> = Result<T, EmitError>;

impl From<EmitError> for TaskError {
    #[inline]
    fn from(error: EmitError) -> Self {
        TaskError::Emit(error)
    }
}
