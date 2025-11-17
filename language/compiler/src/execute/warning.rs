use dyst_dir::{NodeIdAny, Session};

use crate::{CompileWarning, CompilerStage};

/// Warning when executing something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ExecuteWarning {
    /// Complex expression.
    ComplexExpression { node: NodeIdAny } = 1,
}

impl ExecuteWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::ComplexExpression { .. } => 1,
        }
    }

    /// Get the node id of the warning.
    pub fn node_id(&self) -> Option<NodeIdAny> {
        match self {
            Self::ComplexExpression { node, .. } => Some(*node),
        }
    }

    /// Get the message of the warning.
    pub fn message<'a>(&self, _session: &'a Session<'a>) -> String {
        match self {
            Self::ComplexExpression { .. } => "complex expression".to_string(),
        }
    }
}

impl std::fmt::Display for ExecuteWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecuteWarning")
            .field(
                "code",
                &format!("{}W{:03}", CompilerStage::Execute.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<ExecuteWarning> for CompileWarning {
    fn from(warning: ExecuteWarning) -> Self {
        CompileWarning::Execute(warning)
    }
}
