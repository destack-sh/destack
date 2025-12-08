use destack_dir::GlobalNodeIdAny;

use crate::{DiagnosticAnchor, TaskPhase, TaskWarning};

use destack_workspace::Program;

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

    /// Get the anchor of the warning.
    pub fn anchor(&self) -> DiagnosticAnchor {
        match self {
            Self::InscrutableType { node, .. } => DiagnosticAnchor::Node(*node),
            Self::IgnoredHint { node, .. } => DiagnosticAnchor::Node(*node),
            Self::SkippedOptimization { node, .. } => DiagnosticAnchor::Node(*node),
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
                &format!("W{}{:03}", TaskPhase::Optimize.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<OptimizeWarning> for TaskWarning {
    fn from(warning: OptimizeWarning) -> Self {
        TaskWarning::Optimize(warning)
    }
}
