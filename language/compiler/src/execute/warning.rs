use destack_dir::{GlobalNodeIdAny, Program};

use crate::{Phase, TaskWarning};

/// Warning when executing something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ExecuteWarning {
    /// Complex node.
    ComplexNode { node: GlobalNodeIdAny },
    /// Slow evaluation.
    SlowEvaluation { node: GlobalNodeIdAny },
}

impl ExecuteWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::ComplexNode { .. } => 1,
            Self::SlowEvaluation { .. } => 2,
        }
    }

    /// Get the node of the warning.
    pub fn node(&self) -> GlobalNodeIdAny {
        match self {
            Self::ComplexNode { node, .. } => *node,
            Self::SlowEvaluation { node, .. } => *node,
        }
    }

    /// Get the message of the warning.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::ComplexNode { .. } => "complex node".to_string(),
            Self::SlowEvaluation { .. } => "slow evaluation".to_string(),
        }
    }
}

impl std::fmt::Display for ExecuteWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecuteWarning")
            .field(
                "code",
                &format!("W{}{:03}", Phase::Execute.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<ExecuteWarning> for TaskWarning {
    fn from(warning: ExecuteWarning) -> Self {
        TaskWarning::Execute(warning)
    }
}
