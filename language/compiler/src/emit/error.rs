use std::path::PathBuf;

use destack_dir::GlobalNodeIdAny;

use crate::{TaskDependency, TaskDependencyError, TaskError, TaskPhase};

use destack_workspace::Program;

/// Error when emitting output.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum EmitError {
    /// Wait for task dependency.
    Yield { dependency: TaskDependency },
    /// Yield dependency has failed.
    UnsatisfiedDependency { dependency: TaskDependency },
    /// Target not found in package.
    TargetNotFound {
        node: GlobalNodeIdAny,
        target: String,
    },
    /// No output path configured for target.
    NoOutputPath {
        node: GlobalNodeIdAny,
        target: String,
    },
    /// Failed to create output directory.
    CreateDirectoryFailed {
        node: GlobalNodeIdAny,
        path: PathBuf,
        message: Option<String>,
    },
    /// Failed to write output file.
    WriteFailed {
        node: GlobalNodeIdAny,
        path: PathBuf,
        message: Option<String>,
    },
    /// Permission denied when writing output.
    PermissionDenied {
        node: GlobalNodeIdAny,
        path: PathBuf,
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
            Self::NoOutputPath { .. } => 3,
            Self::CreateDirectoryFailed { .. } => 4,
            Self::WriteFailed { .. } => 5,
            Self::PermissionDenied { .. } => 6,
        }
    }

    /// Get the node of the error.
    pub fn node(&self) -> GlobalNodeIdAny {
        match self {
            Self::Yield { dependency } => dependency.node(),
            Self::UnsatisfiedDependency { dependency } => dependency.node(),
            Self::TargetNotFound { node, .. } => *node,
            Self::NoOutputPath { node, .. } => *node,
            Self::CreateDirectoryFailed { node, .. } => *node,
            Self::WriteFailed { node, .. } => *node,
            Self::PermissionDenied { node, .. } => *node,
        }
    }

    /// Get the message of the error.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::Yield { .. } => "pending dependency".to_string(),
            Self::UnsatisfiedDependency { .. } => "unsatisfied dependency".to_string(),
            Self::TargetNotFound { target, .. } => format!("target not found: {target}"),
            Self::NoOutputPath { target, .. } => {
                format!("no output path configured for target: {target}")
            }
            Self::CreateDirectoryFailed { path, message, .. } => {
                let base = format!("failed to create directory: {}", path.display());
                if let Some(msg) = message {
                    format!("{base}: {msg}")
                } else {
                    base
                }
            }
            Self::WriteFailed { path, message, .. } => {
                let base = format!("failed to write file: {}", path.display());
                if let Some(msg) = message {
                    format!("{base}: {msg}")
                } else {
                    base
                }
            }
            Self::PermissionDenied { path, .. } => {
                format!("permission denied: {}", path.display())
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
