use dyst_dir::{LocalNodeIdAny, Session};

use crate::{CompileError, CompileStage};

/// Error when optimizing something into the compiler.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum OptimizeError {
    /// Optimization is impossible for this node.
    UnsupportedNode { node: LocalNodeIdAny },
    /// Unsupported optimization.
    UnsupportedOptimization { node: LocalNodeIdAny },
    /// Undefined behavior possible.
    PossibleUndefinedBehavior {
        node: LocalNodeIdAny,
        behavior: String,
    },
}

impl OptimizeError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::UnsupportedNode { .. } => 1,
            Self::UnsupportedOptimization { .. } => 2,
            Self::PossibleUndefinedBehavior { .. } => 3,
        }
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> Option<LocalNodeIdAny> {
        match self {
            Self::UnsupportedNode { node, .. } => Some(*node),
            Self::UnsupportedOptimization { node, .. } => Some(*node),
            Self::PossibleUndefinedBehavior { node, .. } => Some(*node),
        }
    }

    /// Get the message of the error.
    pub fn message<'a>(&self, _session: &'a Session<'a>) -> String {
        match self {
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
                &format!("{}E{:03}", CompileStage::Optimize.letter(), self.sub_code()),
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
