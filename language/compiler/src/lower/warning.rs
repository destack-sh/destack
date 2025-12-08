use destack_dir::GlobalNodeIdAny;

use crate::{DiagnosticAnchor, TaskPhase, TaskWarning};

use destack_workspace::Program;

/// Warning when optimizing something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum LowerWarning {
    /// Complex type in target language.
    ComplexType { node: GlobalNodeIdAny },
    /// This feature will be emulated slowly on this target.
    SlowEmulation {
        node: GlobalNodeIdAny,
        feature: String,
    },
}

impl LowerWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::ComplexType { .. } => 1,
            Self::SlowEmulation { .. } => 2,
        }
    }

    /// Get the anchor of the warning.
    pub fn anchor(&self) -> DiagnosticAnchor {
        match self {
            Self::ComplexType { node, .. } => DiagnosticAnchor::Node(*node),
            Self::SlowEmulation { node, .. } => DiagnosticAnchor::Node(*node),
        }
    }

    /// Get the message of the warning.
    pub fn message(&self, _program: &Program) -> String {
        match self {
            Self::ComplexType { .. } => "complex type in target".to_string(),
            Self::SlowEmulation { .. } => "slow emulation in target".to_string(),
        }
    }
}

impl std::fmt::Display for LowerWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LowerWarning")
            .field(
                "code",
                &format!("W{}{:03}", TaskPhase::Lower.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<LowerWarning> for TaskWarning {
    fn from(warning: LowerWarning) -> Self {
        TaskWarning::Lower(warning)
    }
}
