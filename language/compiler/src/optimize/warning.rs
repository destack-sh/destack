use crate::DiagnosticAnchor;
use destack_artifact_macros::Diagnostic;

/// Warnings during the optimize phase.
#[derive(Debug, Clone, PartialEq, Diagnostic)]
#[diagnostic(severity = Warning, phase = Optimize)]
pub enum OptimizeWarning {
    /// Owned value created but never used.
    #[diagnostic(id = "unused-owned-value", message = "owned value is never used")]
    UnusedOwnedValue { anchor: DiagnosticAnchor },

    /// Owned value dropped immediately after creation without being used.
    #[diagnostic(
        id = "immediately-dropped",
        message = "value created and immediately dropped"
    )]
    ImmediatelyDropped { anchor: DiagnosticAnchor },

    /// Class call could not be devirtualized.
    #[diagnostic(id = "cannot-devirtualize", message = "cannot devirtualize: {reason}")]
    CannotDevirtualize {
        anchor: DiagnosticAnchor,
        reason: String,
    },

    /// Bounds check could not be eliminated.
    #[diagnostic(
        id = "cannot-eliminate-bounds-check",
        message = "cannot eliminate bounds check: {reason}"
    )]
    CannotEliminateBoundsCheck {
        anchor: DiagnosticAnchor,
        reason: String,
    },

    /// Null check could not be eliminated.
    #[diagnostic(
        id = "cannot-eliminate-null-check",
        message = "cannot eliminate null check: {reason}"
    )]
    CannotEliminateNullCheck {
        anchor: DiagnosticAnchor,
        reason: String,
    },

    /// Function could not be inlined.
    #[diagnostic(id = "cannot-inline", message = "cannot inline: {reason}")]
    CannotInline {
        anchor: DiagnosticAnchor,
        reason: String,
    },

    /// Allocation could not be promoted to stack.
    #[diagnostic(
        id = "cannot-stack-promote",
        message = "cannot stack-promote: {reason}"
    )]
    CannotStackPromote {
        anchor: DiagnosticAnchor,
        reason: String,
    },

    /// Loop optimization failed or was skipped.
    #[diagnostic(id = "loop-not-optimized", message = "loop not optimized: {reason}")]
    LoopNotOptimized {
        anchor: DiagnosticAnchor,
        reason: String,
    },

    /// Optimization hint annotation was ignored.
    #[diagnostic(id = "ignored-hint", message = "optimization hint ignored: {reason}")]
    IgnoredHint {
        anchor: DiagnosticAnchor,
        reason: String,
    },

    /// Optimization was skipped for this node.
    #[diagnostic(
        id = "skipped-optimization",
        message = "optimization skipped: {reason}"
    )]
    SkippedOptimization {
        anchor: DiagnosticAnchor,
        reason: String,
    },
}
