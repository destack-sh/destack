use destack_dir::GlobalNodeIdAny;

use crate::{DiagnosticAnchor, TaskDependency, TaskDependencyError, TaskError, TaskPhase};

use destack_workspace::Program;

/// Error when optimizing something into the compiler.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum OptimizeError {
    /// Wait for task dependency.
    Yield { dependency: TaskDependency },
    /// Yield dependency has failed.
    UnsatisfiedDependency { dependency: TaskDependency },
    /// Optimization is impossible for this node.
    UnsupportedConstruct { node: GlobalNodeIdAny },
    /// Unsupported optimization.
    UnsupportedOptimization { node: GlobalNodeIdAny },
    /// Undefined behavior possible.
    PossibleUndefinedBehavior {
        node: GlobalNodeIdAny,
        behavior: String,
    },
}

impl From<TaskDependencyError> for OptimizeError {
    fn from(e: TaskDependencyError) -> Self {
        match e {
            TaskDependencyError::NotReady { dependency } => Self::Yield { dependency },
            TaskDependencyError::Failed { dependency } => {
                Self::UnsatisfiedDependency { dependency }
            }
        }
    }
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
            Self::UnsatisfiedDependency { .. } => 1,
            Self::UnsupportedConstruct { .. } => 2,
            Self::UnsupportedOptimization { .. } => 3,
            Self::PossibleUndefinedBehavior { .. } => 4,
        }
    }

    /// Get the anchor of the error.
    pub fn anchor(&self) -> DiagnosticAnchor {
        match self {
            Self::Yield { dependency } => dependency.anchor(),
            Self::UnsatisfiedDependency { dependency } => dependency.anchor(),
            Self::UnsupportedConstruct { node, .. } => DiagnosticAnchor::Node(*node),
            Self::UnsupportedOptimization { node, .. } => DiagnosticAnchor::Node(*node),
            Self::PossibleUndefinedBehavior { node, .. } => DiagnosticAnchor::Node(*node),
        }
    }

    /// Get the message of the error.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::Yield { .. } => "pending dependency".to_string(),
            Self::UnsatisfiedDependency { .. } => "unsatisfied dependency".to_string(),
            Self::UnsupportedConstruct { .. } => "unsupported construct".to_string(),
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
                &format!("E{}{:03}", TaskPhase::Optimize.letter(), self.sub_code()),
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
