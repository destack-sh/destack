use dyst_dir::{LocalNodeIdAny, ModuleId, Session};

use crate::{CompileError, CompileStage};

/// Error when binding something into the compiler.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum BindError {
    /// Module not found.
    ModuleNotFound { module: ModuleId },
    /// Unsupported node.
    UnsupportedNode { node: LocalNodeIdAny },
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
            Self::ModuleNotFound { .. } => 1,
            Self::UnsupportedNode { .. } => 2,
        }
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> Option<LocalNodeIdAny> {
        match self {
            Self::ModuleNotFound { .. } => None,
            Self::UnsupportedNode { node, .. } => Some(*node),
        }
    }

    /// Get the message of the error.
    pub fn message<'a>(&self, _session: &'a Session<'a>) -> String {
        match self {
            Self::ModuleNotFound { .. } => "module not found".to_string(),
            Self::UnsupportedNode { .. } => "unsupported node".to_string(),
        }
    }
}

impl std::fmt::Display for BindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BindError")
            .field(
                "code",
                &format!("{}E{:03}", CompileStage::Bind.letter(), self.sub_code()),
            )
            .finish()
    }
}
