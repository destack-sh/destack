use destack_dir::GlobalNodeIdAny;

use crate::{TaskDependency, TaskError, TaskPhase};

use destack_workspace::Program;

/// Error when elaborateing something into the compiler.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ElaborateError {
    /// Wait for task dependency.
    Yield { dependency: TaskDependency },
    /// Yield dependency has failed.
    UnsatisfiedDependency { dependency: TaskDependency },
    /// Unsupported node.
    UnsupportedConstruct { node: GlobalNodeIdAny },
}

impl TryFrom<ElaborateError> for TaskDependency {
    type Error = ElaborateError;

    fn try_from(error: ElaborateError) -> Result<Self, Self::Error> {
        match error {
            ElaborateError::Yield { dependency } => Ok(dependency),
            _ => Err(error),
        }
    }
}

impl ElaborateError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Yield { .. } => 0,
            Self::UnsatisfiedDependency { .. } => 1,
            Self::UnsupportedConstruct { .. } => 2,
        }
    }

    /// Get the node of the error.
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

impl std::fmt::Display for ElaborateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ElaborateError")
            .field(
                "code",
                &format!("E{}{:03}", TaskPhase::Elaborate.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<ElaborateError> for TaskError {
    #[inline]
    fn from(error: ElaborateError) -> Self {
        TaskError::Elaborate(error)
    }
}

pub type ElaborateResult<T> = Result<T, ElaborateError>;
