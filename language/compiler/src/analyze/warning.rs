use crate::{DiagnosticAnchor, DiagnosticDefinition, TaskWarning};
use destack_compiler_macros::DefineWarning;
use destack_dir::AnchoredGlobalNodeId;
use destack_workspace::Program;

/// Warnings during the analyze phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Analyze)]
pub enum AnalyzeWarning {
    // -------------------------------------------------------------------------
    // 1xx: Control flow warnings
    // -------------------------------------------------------------------------
    /// Non-exhaustive match.
    #[warning(code = "WA100", message = "non-exhaustive match")]
    NonExhaustiveMatch { node: AnchoredGlobalNodeId },

    /// Always-true / always-false conditions.
    #[warning(code = "WA102", message = "constant value condition")]
    ConstantValueCondition { node: AnchoredGlobalNodeId },

    // -------------------------------------------------------------------------
    // 2xx: Pattern warnings
    // -------------------------------------------------------------------------
    /// Redundant patterns (match arms never hit).
    #[warning(code = "WA200", message = "redundant pattern")]
    RedundantPattern { node: AnchoredGlobalNodeId },

    /// Shadowed overload: an earlier overload always matches, so this one is never reached.
    #[warning(
        code = "WA201",
        message = "overload is shadowed by an earlier declaration"
    )]
    ShadowedOverload {
        node: AnchoredGlobalNodeId,
        shadowed_by: AnchoredGlobalNodeId,
    },

    // -------------------------------------------------------------------------
    // 3xx: Unused / ignored
    // -------------------------------------------------------------------------
    /// Unused symbol.
    #[warning(code = "WA300", message = "unused symbol")]
    UnusedSymbol { node: AnchoredGlobalNodeId },

    /// Ignored return value.
    #[warning(code = "WA301", message = "ignored return value")]
    IgnoredReturnValue { node: AnchoredGlobalNodeId },

    // -------------------------------------------------------------------------
    // 4xx: Performance / suspicious
    // -------------------------------------------------------------------------
    /// Large dispatch table (performance warning).
    #[warning(
        code = "WA400",
        message = "complex dynamic dispatch ({size} candidates)"
    )]
    ComplexDynamicDispatch {
        node: AnchoredGlobalNodeId,
        size: usize,
    },

    /// Suspicious narrowing.
    #[warning(code = "WA401", message = "suspicious narrowing")]
    SuspiciousNarrowing { node: AnchoredGlobalNodeId },

    // -------------------------------------------------------------------------
    // 5xx: Type inference
    // -------------------------------------------------------------------------
    /// Exported value type could not be inferred.
    #[warning(code = "WA500", message = "exported value has unknown type")]
    ExportTypeUnknown { node: AnchoredGlobalNodeId },
}
