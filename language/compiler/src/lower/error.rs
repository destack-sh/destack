use destack_dir::GlobalNodeIdAny;

use crate::{DiagnosticAnchor, TaskDependency, TaskDependencyError, TaskError, TaskPhase};

use destack_workspace::Program;

/// Error when lowering something into the compiler.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum LowerError {
    /// Wait for task dependency.
    Yield { dependency: TaskDependency } = 0,
    /// Yield dependency has failed.
    UnsatisfiedDependency { dependency: TaskDependency } = 1,
    /// Unsupported node.
    UnsupportedConstruct { node: GlobalNodeIdAny } = 2,
}

impl From<TaskDependencyError> for LowerError {
    fn from(e: TaskDependencyError) -> Self {
        match e {
            TaskDependencyError::NotReady { dependency } => Self::Yield { dependency },
            TaskDependencyError::Failed { dependency } => {
                Self::UnsatisfiedDependency { dependency }
            }
        }
    }
}

impl TryFrom<LowerError> for TaskDependency {
    type Error = LowerError;

    fn try_from(error: LowerError) -> Result<Self, Self::Error> {
        match error {
            LowerError::Yield { dependency } => Ok(dependency),
            _ => Err(error),
        }
    }
}

impl LowerError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Yield { .. } => 0,
            Self::UnsatisfiedDependency { .. } => 1,
            Self::UnsupportedConstruct { .. } => 2,
        }
    }

    /// Get the anchor of the error.
    pub fn anchor(&self) -> DiagnosticAnchor {
        match self {
            Self::Yield { dependency } => dependency.anchor(),
            Self::UnsatisfiedDependency { dependency } => dependency.anchor(),
            Self::UnsupportedConstruct { node, .. } => DiagnosticAnchor::Node(*node),
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

impl std::fmt::Display for LowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LowerError")
            .field(
                "code",
                &format!("E{}{:03}", TaskPhase::Lower.letter(), self.sub_code()),
            )
            .finish()
    }
}

pub type LowerResult<T> = Result<T, LowerError>;

impl From<LowerError> for TaskError {
    #[inline]
    fn from(error: LowerError) -> Self {
        TaskError::Lower(error)
    }
}
