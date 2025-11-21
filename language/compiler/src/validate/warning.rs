use dyst_dir::{LocalNodeIdAny, Session};

use crate::{CompileStage, CompileWarning};

/// Warning when validating something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ValidateWarning {
    /// Non-exhaustive match.
    NonExhaustiveMatch { node: LocalNodeIdAny },
    /// Always-true / always-false conditions.
    ConstantValueCondition { node: LocalNodeIdAny },
    /// Unreachable code.
    UnreachableCode { node: LocalNodeIdAny },
    /// Redundant patterns (match arms never hit).
    RedundantPattern { node: LocalNodeIdAny },
    /// Suspicious narrowing.
    SuspiciousNarrowing { node: LocalNodeIdAny },
    /// Unused symbol.
    UnusedSymbol { node: LocalNodeIdAny },
    /// Ignored return value.
    IgnoredReturnValue { node: LocalNodeIdAny },
}

impl ValidateWarning {
    /// Get the numeric sub-code of the warning.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::NonExhaustiveMatch { .. } => 1,
            Self::UnreachableCode { .. } => 2,
            Self::ConstantValueCondition { .. } => 3,
            Self::RedundantPattern { .. } => 4,
            Self::SuspiciousNarrowing { .. } => 6,
            Self::UnusedSymbol { .. } => 7,
            Self::IgnoredReturnValue { .. } => 8,
        }
    }

    /// Get the node id of the warning.
    pub fn node_id(&self) -> Option<LocalNodeIdAny> {
        match self {
            Self::NonExhaustiveMatch { node, .. } => Some(*node),
            Self::UnreachableCode { node, .. } => Some(*node),
            Self::ConstantValueCondition { node, .. } => Some(*node),
            Self::RedundantPattern { node, .. } => Some(*node),
            Self::SuspiciousNarrowing { node, .. } => Some(*node),
            Self::UnusedSymbol { node, .. } => Some(*node),
            Self::IgnoredReturnValue { node, .. } => Some(*node),
        }
    }

    /// Get the message of the warning.
    pub fn message<'a>(&self, _session: &'a Session<'a>) -> String {
        match self {
            Self::NonExhaustiveMatch { .. } => "non-exhaustive match".to_string(),
            Self::UnreachableCode { .. } => "unreachable code".to_string(),
            Self::ConstantValueCondition { .. } => "constant value condition".to_string(),
            Self::RedundantPattern { .. } => "redundant pattern".to_string(),
            Self::SuspiciousNarrowing { .. } => "suspicious narrowing".to_string(),
            Self::UnusedSymbol { .. } => "unused symbol".to_string(),
            Self::IgnoredReturnValue { .. } => "ignored return value".to_string(),
        }
    }
}

impl std::fmt::Display for ValidateWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidateWarning")
            .field(
                "code",
                &format!("{}W{:03}", CompileStage::Validate.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<ValidateWarning> for CompileWarning {
    fn from(warning: ValidateWarning) -> Self {
        CompileWarning::Validate(warning)
    }
}
