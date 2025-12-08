use destack_dir::GlobalNodeIdAny;

use crate::{DiagnosticAnchor, TaskPhase, TaskWarning};

use destack_workspace::Program;

/// Warning when analyzing something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum VerifyWarning {
    /// Unsupported node.
    UnsupportedConstruct { node: GlobalNodeIdAny },
}

impl VerifyWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::UnsupportedConstruct { .. } => 1,
        }
    }

    /// Get the anchor of the warning.
    pub fn anchor(&self) -> DiagnosticAnchor {
        match self {
            Self::UnsupportedConstruct { node, .. } => DiagnosticAnchor::Node(*node),
        }
    }

    /// Get the message of the warning.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::UnsupportedConstruct { .. } => "unsupported construct".to_string(),
        }
    }
}

impl std::fmt::Display for VerifyWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerifyWarning")
            .field(
                "code",
                &format!("W{}{:03}", TaskPhase::Verify.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<VerifyWarning> for TaskWarning {
    fn from(warning: VerifyWarning) -> Self {
        TaskWarning::Verify(warning)
    }
}
