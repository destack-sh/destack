use dyst_dir::{GlobalNodeIdAny, Program};

use crate::{CompileWarning, Phase};

/// Warning when analyzing something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum AnalyzeWarning {
    /// Unsupported node.
    UnsupportedNode { node: GlobalNodeIdAny },
}

impl AnalyzeWarning {
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

impl std::fmt::Display for AnalyzeWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalyzeWarning")
            .field(
                "code",
                &format!("W{}{:03}", Phase::Analyze.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<AnalyzeWarning> for CompileWarning {
    fn from(warning: AnalyzeWarning) -> Self {
        CompileWarning::Analyze(warning)
    }
}
