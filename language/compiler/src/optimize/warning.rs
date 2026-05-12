use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;

/// Warnings during the optimize phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Optimize)]
pub enum OptimizeWarning {
    // -------------------------------------------------------------------------
    // 1xx: Aliasing warnings
    // -------------------------------------------------------------------------
    /// Potential aliasing violation (hint mode warning).
    #[diagnostic(code = "WO100", message = "potential aliasing")]
    PotentialAliasingViolation {
        anchor: DiagnosticAnchor,
        existing_borrow: DiagnosticAnchor,
    },

    /// Reference may be invalidated by mutation (hint mode warning).
    #[diagnostic(code = "WO101", message = "reference may be invalidated")]
    PotentialInvalidatedReference {
        anchor: DiagnosticAnchor,
        mutation_at: DiagnosticAnchor,
    },

    /// Borrow may escape its scope (hint mode warning).
    #[diagnostic(code = "WO102", message = "borrow may escape scope")]
    PotentialBorrowEscape { anchor: DiagnosticAnchor },

    /// Declared return lifetime does not cover the returned borrow.
    #[diagnostic(
        code = "WO103",
        message = "return lifetime does not cover returned borrow"
    )]
    PotentialReturnLifetimeMismatch { anchor: DiagnosticAnchor },

    /// Declared return lifetime was ignored for a non borrowed return.
    #[diagnostic(
        code = "WO104",
        message = "return lifetime ignored for non borrowed return"
    )]
    ReturnLifetimeIgnored { anchor: DiagnosticAnchor },

    // -------------------------------------------------------------------------
    // 2xx: Unused value warnings
    // -------------------------------------------------------------------------
    /// Owned value created but never used.
    #[diagnostic(code = "WO200", message = "owned value is never used")]
    UnusedOwnedValue { anchor: DiagnosticAnchor },

    /// Owned value dropped immediately after creation without being used.
    #[diagnostic(code = "WO201", message = "value created and immediately dropped")]
    ImmediatelyDropped { anchor: DiagnosticAnchor },

    // -------------------------------------------------------------------------
    // 3xx: Optimization missed warnings
    // -------------------------------------------------------------------------
    /// Class call could not be devirtualized.
    #[diagnostic(code = "WO300", message = "cannot devirtualize: {reason}")]
    CannotDevirtualize {
        anchor: DiagnosticAnchor,
        reason: String,
    },

    /// Bounds check could not be eliminated.
    #[diagnostic(code = "WO301", message = "cannot eliminate bounds check: {reason}")]
    CannotEliminateBoundsCheck {
        anchor: DiagnosticAnchor,
        reason: String,
    },

    /// Null check could not be eliminated.
    #[diagnostic(code = "WO302", message = "cannot eliminate null check: {reason}")]
    CannotEliminateNullCheck {
        anchor: DiagnosticAnchor,
        reason: String,
    },

    /// Function could not be inlined.
    #[diagnostic(code = "WO303", message = "cannot inline: {reason}")]
    CannotInline {
        anchor: DiagnosticAnchor,
        reason: String,
    },

    /// Allocation could not be promoted to stack.
    #[diagnostic(code = "WO304", message = "cannot stack-promote: {reason}")]
    CannotStackPromote {
        anchor: DiagnosticAnchor,
        reason: String,
    },

    /// Loop optimization failed or was skipped.
    #[diagnostic(code = "WO305", message = "loop not optimized: {reason}")]
    LoopNotOptimized {
        anchor: DiagnosticAnchor,
        reason: String,
    },

    // -------------------------------------------------------------------------
    // 9xx: Hint / skipped warnings
    // -------------------------------------------------------------------------
    /// Optimization hint annotation was ignored.
    #[diagnostic(code = "WO900", message = "optimization hint ignored: {reason}")]
    IgnoredHint {
        anchor: DiagnosticAnchor,
        reason: String,
    },

    /// Optimization was skipped for this node.
    #[diagnostic(code = "WO901", message = "optimization skipped: {reason}")]
    SkippedOptimization {
        anchor: DiagnosticAnchor,
        reason: String,
    },
}
