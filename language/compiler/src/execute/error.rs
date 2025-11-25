use dyst_dir::{GlobalNodeIdAny, Program};

use crate::{CompileError, CompilePhase, CompileTaskWait};

/// Error when evaluating something statically.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ExecuteError {
    /// Wait for other tasks.
    Wait { wait: CompileTaskWait } = 0,
    /// Unsupported node.
    UnsupportedNode { node: GlobalNodeIdAny } = 1,
}

impl ExecuteError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Wait { .. } => 0,
            Self::UnsupportedNode { .. } => 1,
        }
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> Option<GlobalNodeIdAny> {
        match self {
            Self::Wait { wait } => wait.nodes.first().copied(),
            Self::UnsupportedNode { node, .. } => Some(*node),
        }
    }

    /// Get the message of the error.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::Wait { .. } => "wait for task".to_string(),
            Self::UnsupportedNode { .. } => "unsupported node".to_string(),
        }
    }
}

impl std::fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecuteError")
            .field(
                "code",
                &format!("E{}{:03}", CompilePhase::Execute.letter(), self.sub_code()),
            )
            .finish()
    }
}

pub type ExecuteResult<T> = Result<T, ExecuteError>;

impl From<ExecuteError> for CompileError {
    #[inline]
    fn from(error: ExecuteError) -> Self {
        CompileError::Execute(error)
    }
}
