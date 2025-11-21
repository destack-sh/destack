use dyst_dir::{LocalNodeIdAny, Session};

use crate::{CompileWarning, CompileStage};

/// Warning when optimizing something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum LowerWarning {
    /// Complex type in target language.
    ComplexType { node: LocalNodeIdAny },
    /// This feature will be emulated slowly on this target.
    SlowEmulation { node: LocalNodeIdAny, feature: String },
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

    /// Get the node id of the warning.
    pub fn node_id(&self) -> Option<LocalNodeIdAny> {
        match self {
            Self::ComplexType { node, .. } => Some(*node),
            Self::SlowEmulation { node, .. } => Some(*node),
        }
    }

    /// Get the message of the warning.
    pub fn message<'a>(&self, _session: &'a Session<'a>) -> String {
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
                &format!("{}W{:03}", CompileStage::Lower.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<LowerWarning> for CompileWarning {
    fn from(warning: LowerWarning) -> Self {
        CompileWarning::Lower(warning)
    }
}
