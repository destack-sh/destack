use destack_dir::GlobalNodeIdAny;

use crate::{TaskDependency, TaskDependencyError, TaskError, TaskPhase};

use destack_workspace::Program;

/// Error when verifying something into the compiler.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum VerifyError {
    /// Wait for task dependency.
    Yield { dependency: TaskDependency },
    /// Yield dependency has failed.
    UnsatisfiedDependency { dependency: TaskDependency },
    /// Unsupported node.
    UnsupportedConstruct { node: GlobalNodeIdAny },
}

impl From<TaskDependencyError> for VerifyError {
    fn from(e: TaskDependencyError) -> Self {
        match e {
            TaskDependencyError::NotReady { dependency } => Self::Yield { dependency },
            TaskDependencyError::Failed { dependency } => {
                Self::UnsatisfiedDependency { dependency }
            }
        }
    }
}

impl TryFrom<VerifyError> for TaskDependency {
    type Error = VerifyError;

    fn try_from(error: VerifyError) -> Result<Self, Self::Error> {
        match error {
            VerifyError::Yield { dependency } => Ok(dependency),
            _ => Err(error),
        }
    }
}

impl VerifyError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Yield { .. } => 0,
            Self::UnsatisfiedDependency { .. } => 1,
            Self::UnsupportedConstruct { .. } => 2,
        }
    }

    /// Get the node id of the error.
    pub fn node(&self) -> GlobalNodeIdAny {
        match self {
            Self::Yield { dependency } => dependency.node(),
            Self::UnsatisfiedDependency { dependency } => dependency.node(),
            Self::UnsupportedConstruct { node, .. } => *node,
        }
    }

    /// Get the message of the error.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::Yield { .. } => "pending dependency".to_string(),
            Self::UnsatisfiedDependency { .. } => "unsatisfied dependency".to_string(),
            Self::UnsupportedConstruct { .. } => "unsupported construct".to_string(),
        }
    }
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerifyError")
            .field(
                "code",
                &format!("E{}{:03}", TaskPhase::Verify.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<VerifyError> for TaskError {
    #[inline]
    fn from(error: VerifyError) -> Self {
        TaskError::Verify(error)
    }
}

pub type VerifyResult<T> = Result<T, VerifyError>;
