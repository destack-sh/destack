use destack_dir::GlobalNodeIdAny;
use destack_workspace::Program;

use crate::{DiagnosticAnchor, TaskPhase, TaskWarning};

/// Warning when validating something.
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum AnalyzeWarning {
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
    /// Large dispatch table (performance warning).
    ComplexDynamicDispatch { node: GlobalNodeIdAny, size: usize },
    /// Shadowed overload: an earlier overload always matches, so this one is never reached.
    ShadowedOverload {
        node: GlobalNodeIdAny,
        shadowed_by: GlobalNodeIdAny,
    },
}

impl AnalyzeWarning {
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
            Self::ComplexDynamicDispatch { .. } => 9,
            Self::ShadowedOverload { .. } => 10,
        }
    }

    /// Get the anchor of the warning.
    pub fn anchor(&self) -> DiagnosticAnchor {
        match self {
            Self::NonExhaustiveMatch { node, .. } => DiagnosticAnchor::Node(*node),
            Self::UnreachableCode { node, .. } => DiagnosticAnchor::Node(*node),
            Self::ConstantValueCondition { node, .. } => DiagnosticAnchor::Node(*node),
            Self::RedundantPattern { node, .. } => DiagnosticAnchor::Node(*node),
            Self::SuspiciousNarrowing { node, .. } => DiagnosticAnchor::Node(*node),
            Self::UnusedSymbol { node, .. } => DiagnosticAnchor::Node(*node),
            Self::IgnoredReturnValue { node, .. } => DiagnosticAnchor::Node(*node),
            Self::ComplexDynamicDispatch { node, .. } => DiagnosticAnchor::Node(*node),
            Self::ShadowedOverload { node, .. } => DiagnosticAnchor::Node(*node),
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
            Self::ComplexDynamicDispatch { size, .. } => {
                format!("complex dynamic dispatch ({size} candidates)")
            }
            Self::ShadowedOverload { .. } => {
                "overload is shadowed by an earlier declaration".to_string()
            }
        }
    }
}

impl std::fmt::Display for AnalyzeWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalyzeWarning")
            .field(
                "code",
                &format!("W{}{:03}", TaskPhase::Analyze.letter(), self.sub_code()),
            )
            .finish()
    }
}

impl From<AnalyzeWarning> for TaskWarning {
    fn from(warning: AnalyzeWarning) -> Self {
        TaskWarning::Analyze(warning)
    }
}
