use dyst_dir::{GlobalNodeIdAny, Program};

use crate::{CompileError, CompilePhase, CompileTaskDependency};

/// Error when elaborateing something into the compiler.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ElaborateError {
    /// Wait for other tasks.
    Wait { wait: CompileTaskDependency },
    /// Unsupported node.
    UnsupportedNode { node: GlobalNodeIdAny },
}

impl ElaborateError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Wait { .. } => 0,
            Self::UnsupportedNode { .. } => 2,
        }
    }

    /// Get the node id of the error.
    pub fn node_id(&self) -> Option<GlobalNodeIdAny> {
        match self {
            Self::Wait { wait } => wait.first_node(),
            Self::UnsupportedNode { node, .. } => Some(*node),
        }
    }

    /// Get the message of the error.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::Wait { .. } => "wait for task".to_string(),
            Self::UnsupportedNode { .. } => "unsupported node".to_string(),
        }
    }
}

impl std::fmt::Display for ElaborateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ElaborateError")
            .field(
                "code",
                &format!(
                    "E{}{:03}",
                    CompilePhase::Elaborate.letter(),
                    self.sub_code()
                ),
            )
            .finish()
    }
}

impl From<ElaborateError> for CompileError {
    #[inline]
    fn from(error: ElaborateError) -> Self {
        CompileError::Elaborate(error)
    }
}

pub type ElaborateResult<T> = Result<T, ElaborateError>;
