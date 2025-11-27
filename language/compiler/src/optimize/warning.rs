use dyst_dir::{GlobalNodeIdAny, Program};

use crate::{Phase, CompileWarning};

/// Warning when optimizing something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum OptimizeWarning {
    /// Inscrutable type for an expression.
    InscrutableType { node: GlobalNodeIdAny },
    /// Hint ignored.
    IgnoredHint {
        node: GlobalNodeIdAny,
        message: Option<String>,
    },
    /// Optimization skipped.
    SkippedOptimization {
        node: GlobalNodeIdAny,
        message: Option<String>,
    },
}

impl OptimizeWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::InscrutableType { .. } => 1,
            Self::IgnoredHint { .. } => 2,
            Self::SkippedOptimization { .. } => 3,
        }
    }

    /// Get the node id of the warning.
    pub fn node_id(&self) -> Option<GlobalNodeIdAny> {
        match self {
            Self::InscrutableType { node, .. } => Some(*node),
            Self::IgnoredHint { node, .. } => Some(*node),
            Self::SkippedOptimization { node, .. } => Some(*node),
        }
    }

    /// Get the message of the warning.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::InscrutableType { .. } => "inscrutable type".to_string(),
            Self::IgnoredHint { message, .. } => message
                .as_ref()
                .cloned()
                .unwrap_or("ignored hint".to_string()),
            Self::SkippedOptimization { message, .. } => message
                .as_ref()
                .cloned()
                .unwrap_or("optimization skipped".to_string()),
        }
    }
}

impl std::fmt::Display for OptimizeWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OptimizeWarning")
            .field(
                "code",
                &format!("W{}{:03}", Phase::Optimize.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<OptimizeWarning> for CompileWarning {
    fn from(warning: OptimizeWarning) -> Self {
        CompileWarning::Optimize(warning)
    }
}
