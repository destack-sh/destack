use dyst_dir::{GlobalNodeIdAny, Program};

use crate::{CompilePhase, CompileWarning};

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

    /// Get the node id of the warning.
    pub fn node_id(&self) -> Option<GlobalNodeIdAny> {
        match self {
            Self::UnsupportedNode { node, .. } => Some(*node),
        }
    }

    /// Get the message of the warning.
    pub fn message<'a>(&self, _program: &'a Program<'a>) -> String {
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
                &format!(
                    "{}W{:03}",
                    CompilePhase::Elaborate.letter(),
                    self.sub_code()
                ),
            )
            .finish()
    }
}

impl From<ElaborateWarning> for CompileWarning {
    fn from(warning: ElaborateWarning) -> Self {
        CompileWarning::Elaborate(warning)
    }
}
