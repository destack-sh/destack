use destack_compiler_macros::DefineWarning;
use destack_mir as mir;
use destack_workspace::Program;

use crate::{DiagnosticAnchor, DiagnosticDefinition, TaskWarning};

/// Warnings during the optimize phase.
#[derive(Debug, Clone, PartialEq, DefineWarning)]
#[phase(Optimize)]
pub enum OptimizeWarning {
    // -------------------------------------------------------------------------
    // 1xx: Aliasing warnings
    // -------------------------------------------------------------------------
    /// Potential aliasing violation (hint mode warning).
    #[warning(code = "WO100", message = "potential aliasing")]
    PotentialAliasingViolation {
        node: mir::AnchoredGlobalNodeId,
        existing_borrow: mir::AnchoredGlobalNodeId,
    },

    /// Reference may be invalidated by mutation (hint mode warning).
    #[warning(code = "WO101", message = "reference may be invalidated")]
    PotentialInvalidatedReference {
        node: mir::AnchoredGlobalNodeId,
        mutation_at: mir::AnchoredGlobalNodeId,
    },

    /// Borrow may escape its scope (hint mode warning).
    #[warning(code = "WO102", message = "borrow may escape scope")]
    PotentialBorrowEscape { node: mir::AnchoredGlobalNodeId },

    /// Lifetime annotation does not cover the returned borrow.
    #[warning(
        code = "WO103",
        message = "lifetime annotation does not cover returned borrow"
    )]
    PotentialLifetimeAnnotationMismatch { node: mir::AnchoredGlobalNodeId },

    /// Lifetime annotation was ignored for a non borrowed return.
    #[warning(
        code = "WO104",
        message = "lifetime annotation ignored for non borrowed return"
    )]
    LifetimeAnnotationIgnored { node: mir::AnchoredGlobalNodeId },

    // -------------------------------------------------------------------------
    // 2xx: Unused value warnings
    // -------------------------------------------------------------------------
    /// Owned value created but never used.
    #[warning(code = "WO200", message = "owned value is never used")]
    UnusedOwnedValue { node: mir::AnchoredGlobalNodeId },

    /// Owned value dropped immediately after creation without being used.
    #[warning(code = "WO201", message = "value created and immediately dropped")]
    ImmediatelyDropped { node: mir::AnchoredGlobalNodeId },

    // -------------------------------------------------------------------------
    // 3xx: Optimization missed warnings
    // -------------------------------------------------------------------------
    /// Virtual call could not be devirtualized.
    #[warning(code = "WO300", message = "cannot devirtualize: {reason}")]
    CannotDevirtualize {
        node: mir::AnchoredGlobalNodeId,
        reason: String,
    },

    /// Bounds check could not be eliminated.
    #[warning(code = "WO301", message = "cannot eliminate bounds check: {reason}")]
    CannotEliminateBoundsCheck {
        node: mir::AnchoredGlobalNodeId,
        reason: String,
    },

    /// Null check could not be eliminated.
    #[warning(code = "WO302", message = "cannot eliminate null check: {reason}")]
    CannotEliminateNullCheck {
        node: mir::AnchoredGlobalNodeId,
        reason: String,
    },

    /// Function could not be inlined.
    #[warning(code = "WO303", message = "cannot inline: {reason}")]
    CannotInline {
        node: mir::AnchoredGlobalNodeId,
        reason: String,
    },

    /// Allocation could not be promoted to stack.
    #[warning(code = "WO304", message = "cannot stack-promote: {reason}")]
    CannotStackPromote {
        node: mir::AnchoredGlobalNodeId,
        reason: String,
    },

    /// Loop optimization failed or was skipped.
    #[warning(code = "WO305", message = "loop not optimized: {reason}")]
    LoopNotOptimized {
        node: mir::AnchoredGlobalNodeId,
        reason: String,
    },

    // -------------------------------------------------------------------------
    // 9xx: Hint / skipped warnings
    // -------------------------------------------------------------------------
    /// Optimization hint annotation was ignored.
    #[warning(code = "WO900", message = "optimization hint ignored: {reason}")]
    IgnoredHint {
        node: mir::AnchoredGlobalNodeId,
        reason: String,
    },

    /// Optimization was skipped for this node.
    #[warning(code = "WO901", message = "optimization skipped: {reason}")]
    SkippedOptimization {
        node: mir::AnchoredGlobalNodeId,
        reason: String,
    },
}
