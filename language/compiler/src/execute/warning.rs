use dyst_dir::{GlobalNodeIdAny, Session};

use crate::{CompilePhase, CompileWarning};

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

    /// Get the node id of the warning.
    pub fn node_id(&self) -> Option<GlobalNodeIdAny> {
        match self {
            Self::ComplexNode { node, .. } => Some(*node),
            Self::SlowEvaluation { node, .. } => Some(*node),
        }
    }

    /// Get the message of the warning.
    pub fn message<'a>(&self, _session: &'a Session<'a>) -> String {
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
                &format!("{}W{:03}", CompilePhase::Execute.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<ExecuteWarning> for CompileWarning {
    fn from(warning: ExecuteWarning) -> Self {
        CompileWarning::Execute(warning)
    }
}
