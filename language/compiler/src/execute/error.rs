use dyst_dir::{NodeIdAny, Session};

use crate::{CompileError, CompilerStage};

/// Error when evaluating something statically.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum ExecuteError {
    /// Unsupported node.
    UnsupportedNode { node: NodeIdAny } = 1,
}

impl ExecuteError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::UnsupportedNode { .. } => 1,
        }
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> Option<NodeIdAny> {
        match self {
            Self::UnsupportedNode { node, .. } => Some(*node),
        }
    }

    /// Get the message of the error.
    pub fn message<'a>(&self, _session: &'a Session<'a>) -> String {
        match self {
            Self::UnsupportedNode { .. } => "unsupported node".to_string(),
        }
    }
}

impl std::fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecuteError")
            .field(
                "code",
                &format!("{}E{:03}", CompilerStage::Execute.letter(), self.sub_code()),
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
