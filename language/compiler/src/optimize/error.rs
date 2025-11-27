use dyst_dir::{GlobalNodeIdAny, Program};

use crate::{Phase, TaskDependency, TaskError};

/// Error when optimizing something into the compiler.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum OptimizeError {
    /// Wait for task dependency.
    Yield { dependency: TaskDependency },
    /// Yield dependency has failed.
    YieldFailed { dependency: TaskDependency },
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

impl TryFrom<OptimizeError> for TaskDependency {
    type Error = OptimizeError;

    fn try_from(error: OptimizeError) -> Result<Self, Self::Error> {
        match error {
            OptimizeError::Yield { dependency } => Ok(dependency),
            _ => Err(error),
        }
    }
}

impl OptimizeError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Yield { .. } => 0,
            Self::YieldFailed { .. } => 1,
            Self::UnsupportedNode { .. } => 2,
            Self::UnsupportedOptimization { .. } => 3,
            Self::PossibleUndefinedBehavior { .. } => 4,
        }
    }

    /// Get the node of the error.
    pub fn node(&self) -> GlobalNodeIdAny {
        match self {
            Self::Yield { dependency } => dependency.node(),
            Self::YieldFailed { dependency } => dependency.node(),
            Self::UnsupportedNode { node, .. } => *node,
            Self::UnsupportedOptimization { node, .. } => *node,
            Self::PossibleUndefinedBehavior { node, .. } => *node,
        }
    }

    /// Get the message of the error.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::Yield { .. } => "pending dependency".to_string(),
            Self::YieldFailed { .. } => "unsatisfied dependency".to_string(),
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
                &format!("E{}{:03}", Phase::Optimize.letter(), self.sub_code()),
            )
            .finish()
    }
}

pub type OptimizeResult<T> = Result<T, OptimizeError>;

impl From<OptimizeError> for TaskError {
    #[inline]
    fn from(error: OptimizeError) -> Self {
        TaskError::Optimize(error)
    }
}
