use dyst_dir::{GlobalNodeIdAny, Program};

use crate::{CompileError, CompilePhase, CompileTaskWait};

/// Error when binding something into the compiler.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum BindError {
    /// Wait for other tasks.
    Wait { wait: CompileTaskWait },
    /// Unsupported node.
    UnsupportedNode { node: GlobalNodeIdAny },
}

pub type BindResult<T> = Result<T, BindError>;

impl From<BindError> for CompileError {
    #[inline]
    fn from(error: BindError) -> Self {
        CompileError::Bind(error)
    }
}

impl BindError {
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
            Self::Wait { wait, .. } => wait.nodes.first().copied(),
            Self::UnsupportedNode { node } => Some(*node),
        }
    }

    /// Get the message of the error.
    pub fn message<'a>(&self, _program: &'a Program<'a>) -> String {
        match self {
            Self::Wait { wait, .. } => {
                format!("wait for {} tasks", wait.tasks.len())
            }
            Self::UnsupportedNode { .. } => "unsupported node".to_string(),
        }
    }
}

impl std::fmt::Display for BindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BindError")
            .field(
                "code",
                &format!("E{}{:03}", CompilePhase::Bind.letter(), self.sub_code()),
            )
            .finish()
    }
}
