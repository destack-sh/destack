use dyst_dir::{GlobalNodeIdAny, Program};

use crate::{CompileError, CompilePhase, CompileTaskWait};

/// Error when optimizing something into the compiler.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum OptimizeError {
    /// Wait for other tasks.
    Wait { wait: CompileTaskWait },
    /// Optimization is impossible for this node.
    UnsupportedNode { node: GlobalNodeIdAny },
    /// Unsupported optimization.
    UnsupportedOptimization { node: GlobalNodeIdAny },
    /// Undefined behavior possible.
    PossibleUndefinedBehavior {
        node: GlobalNodeIdAny,
        behavior: String,
    },
}

impl OptimizeError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Wait { .. } => 0,
            Self::UnsupportedNode { .. } => 1,
            Self::UnsupportedOptimization { .. } => 2,
            Self::PossibleUndefinedBehavior { .. } => 3,
        }
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> Option<GlobalNodeIdAny> {
        match self {
            Self::Wait { wait, .. } => wait.nodes.first().copied(),
            Self::UnsupportedNode { node, .. } => Some(*node),
            Self::UnsupportedOptimization { node, .. } => Some(*node),
            Self::PossibleUndefinedBehavior { node, .. } => Some(*node),
        }
    }

    /// Get the message of the error.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::Wait { wait, .. } => {
                format!("wait for {} tasks", wait.tasks.len())
            }
            Self::UnsupportedNode { .. } => "unsupported node".to_string(),
            Self::UnsupportedOptimization { .. } => "unsupported optimization".to_string(),
            Self::PossibleUndefinedBehavior { .. } => "possible undefined behavior".to_string(),
        }
    }
}

impl std::fmt::Display for OptimizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OptimizeError")
            .field(
                "code",
                &format!("E{}{:03}", CompilePhase::Optimize.letter(), self.sub_code()),
            )
            .finish()
    }
}

pub type OptimizeResult<T> = Result<T, OptimizeError>;

impl From<OptimizeError> for CompileError {
    #[inline]
    fn from(error: OptimizeError) -> Self {
        CompileError::Optimize(error)
    }
}
