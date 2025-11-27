use dyst_dir::{GlobalNodeIdAny, Program};

use crate::{TaskError, Phase, TaskDependency};

/// Error when evaluating something statically.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ExecuteError {
    /// Wait for task dependency.
    Yield { wait: TaskDependency } = 0,
    /// Unsupported node.
    UnsupportedNode { node: GlobalNodeIdAny } = 1,
}

impl TryFrom<ExecuteError> for TaskDependency {
    type Error = ExecuteError;

    fn try_from(error: ExecuteError) -> Result<Self, Self::Error> {
        match error {
            ExecuteError::Yield { wait } => Ok(wait),
            _ => Err(error),
        }
    }
}

impl ExecuteError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Yield { .. } => 0,
            Self::UnsupportedNode { .. } => 1,
        }
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> Option<GlobalNodeIdAny> {
        match self {
            Self::Yield { wait } => wait.first_node(),
            Self::UnsupportedNode { node, .. } => Some(*node),
        }
    }

    /// Get the message of the error.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::Yield { .. } => "unresolved dependency".to_string(),
            Self::UnsupportedNode { .. } => "unsupported node".to_string(),
        }
    }
}

impl std::fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecuteError")
            .field(
                "code",
                &format!("E{}{:03}", Phase::Execute.letter(), self.sub_code()),
            )
            .finish()
    }
}

pub type ExecuteResult<T> = Result<T, ExecuteError>;

impl From<ExecuteError> for TaskError {
    #[inline]
    fn from(error: ExecuteError) -> Self {
        TaskError::Execute(error)
    }
}
