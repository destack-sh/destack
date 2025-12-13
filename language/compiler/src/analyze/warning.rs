use crate::{DiagnosticAnchor, DiagnosticDefinition, TaskWarning};
use destack_compiler_macros::DefineWarning;
use destack_dir::GlobalNodeIdAny;
use destack_workspace::Program;

/// Warnings during the analyze phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Analyze)]
pub enum AnalyzeWarning {
    /// Non-exhaustive match.
    #[warning(code = "WA001", message = "non-exhaustive match")]
    NonExhaustiveMatch { node: GlobalNodeIdAny },

    /// Unreachable code.
    #[warning(code = "WA002", message = "unreachable code")]
    UnreachableCode { node: GlobalNodeIdAny },

    /// Always-true / always-false conditions.
    #[warning(code = "WA003", message = "constant value condition")]
    ConstantValueCondition { node: GlobalNodeIdAny },

    /// Redundant patterns (match arms never hit).
    #[warning(code = "WA004", message = "redundant pattern")]
    RedundantPattern { node: GlobalNodeIdAny },

    /// Suspicious narrowing.
    #[warning(code = "WA006", message = "suspicious narrowing")]
    SuspiciousNarrowing { node: GlobalNodeIdAny },

    /// Unused symbol.
    #[warning(code = "WA007", message = "unused symbol")]
    UnusedSymbol { node: GlobalNodeIdAny },

    /// Ignored return value.
    #[warning(code = "WA008", message = "ignored return value")]
    IgnoredReturnValue { node: GlobalNodeIdAny },

    /// Large dispatch table (performance warning).
    #[warning(
        code = "WA009",
        message = "complex dynamic dispatch ({size} candidates)"
    )]
    ComplexDynamicDispatch { node: GlobalNodeIdAny, size: usize },

    /// Shadowed overload: an earlier overload always matches, so this one is never reached.
    #[warning(
        code = "WA010",
        message = "overload is shadowed by an earlier declaration"
    )]
    ShadowedOverload {
        node: GlobalNodeIdAny,
        shadowed_by: GlobalNodeIdAny,
    },
}
