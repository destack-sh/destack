use destack_dir::{GlobalNodeIdAny, Program};

use crate::{TaskPhase, TaskWarning};

/// Warning when validating something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ElaborateWarning {
    /// Unsupported node.
    UnsupportedNode { node: GlobalNodeIdAny },
}

impl ElaborateWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::UnsupportedNode { .. } => 1,
        }
    }

    /// Get the node of the warning.
    pub fn node(&self) -> GlobalNodeIdAny {
        match self {
            Self::UnsupportedNode { node, .. } => *node,
        }
    }

    /// Get the message of the warning.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::UnsupportedNode { .. } => "unsupported node".to_string(),
        }
    }
}

impl std::fmt::Display for ElaborateWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ElaborateWarning")
            .field(
                "code",
                &format!("W{}{:03}", TaskPhase::Elaborate.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<ElaborateWarning> for TaskWarning {
    fn from(warning: ElaborateWarning) -> Self {
        TaskWarning::Elaborate(warning)
    }
}
