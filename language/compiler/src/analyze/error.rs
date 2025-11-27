use dyst_dir::{GlobalNodeIdAny, Program};

use crate::{TaskError, Phase, TaskDependency};

/// Error when analyzeing something into the compiler.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum AnalyzeError {
    /// Wait for task dependency.
    Yield { wait: TaskDependency },
    /// Unsupported node.
    UnsupportedNode { node: GlobalNodeIdAny },
}

impl TryFrom<AnalyzeError> for TaskDependency {
    type Error = AnalyzeError;

    fn try_from(error: AnalyzeError) -> Result<Self, Self::Error> {
        match error {
            AnalyzeError::Yield { wait } => Ok(wait),
            _ => Err(error),
        }
    }
}

impl AnalyzeError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Yield { .. } => 0,
            Self::UnsupportedNode { .. } => 2,
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

impl std::fmt::Display for AnalyzeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalyzeError")
            .field(
                "code",
                &format!("E{}{:03}", Phase::Analyze.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<AnalyzeError> for TaskError {
    #[inline]
    fn from(error: AnalyzeError) -> Self {
        TaskError::Analyze(error)
    }
}

pub type AnalyzeResult<T> = Result<T, AnalyzeError>;
