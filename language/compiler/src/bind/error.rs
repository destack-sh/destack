use dyst_dir::{NodeIdAny, Session};

use crate::{CompileError, CompilerStage};

/// Error when binding something into the compiler.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum BindError {
    /// Unsupported node.
    UnsupportedNode { node: NodeIdAny },
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

impl std::fmt::Display for BindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BindError")
            .field(
                "code",
                &format!("{}E{:03}", CompilerStage::Bind.letter(), self.sub_code()),
            )
            .finish()
    }
}
