use dyst_dir::{GlobalNodeIdAny, Program};

use crate::{CompileError, CompilePhase, CompileTaskDependency};

/// Error when lowering something into the compiler.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum LowerError {
    /// Wait for task dependency.
    Yield { wait: CompileTaskDependency } = 0,
    /// Unsupported node.
    UnsupportedNode { node: GlobalNodeIdAny } = 1,
}

impl LowerError {
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

impl std::fmt::Display for LowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LowerError")
            .field(
                "code",
                &format!("E{}{:03}", CompilePhase::Lower.letter(), self.sub_code()),
            )
            .finish()
    }
}

pub type LowerResult<T> = Result<T, LowerError>;

impl From<LowerError> for CompileError {
    #[inline]
    fn from(error: LowerError) -> Self {
        CompileError::Lower(error)
    }
}
