use dyst_dir::{GlobalNodeIdAny, Program};

use crate::{TaskWarning, Phase};

/// Warning when validating something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ValidateWarning {
    /// Non-exhaustive match.
    NonExhaustiveMatch { node: GlobalNodeIdAny },
    /// Always-true / always-false conditions.
    ConstantValueCondition { node: GlobalNodeIdAny },
    /// Unreachable code.
    UnreachableCode { node: GlobalNodeIdAny },
    /// Redundant patterns (match arms never hit).
    RedundantPattern { node: GlobalNodeIdAny },
    /// Suspicious narrowing.
    SuspiciousNarrowing { node: GlobalNodeIdAny },
    /// Unused symbol.
    UnusedSymbol { node: GlobalNodeIdAny },
    /// Ignored return value.
    IgnoredReturnValue { node: GlobalNodeIdAny },
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

    /// Get the node of the warning.
    pub fn node(&self) -> GlobalNodeIdAny {
        match self {
            Self::NonExhaustiveMatch { node, .. } => *node,
            Self::UnreachableCode { node, .. } => *node,
            Self::ConstantValueCondition { node, .. } => *node,
            Self::RedundantPattern { node, .. } => *node,
            Self::SuspiciousNarrowing { node, .. } => *node,
            Self::UnusedSymbol { node, .. } => *node,
            Self::IgnoredReturnValue { node, .. } => *node,
        }
    }

    /// Get the message of the warning.
    pub fn message(&self, _program: &Program) -> String {
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
                &format!("W{}{:03}", Phase::Validate.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<ValidateWarning> for TaskWarning {
    fn from(warning: ValidateWarning) -> Self {
        TaskWarning::Validate(warning)
    }
}
