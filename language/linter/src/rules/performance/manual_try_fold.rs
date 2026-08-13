use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer tryFold over folding Results by hand.
    pub MANUAL_TRY_FOLD {
        id: "manual-try-fold",
        summary: "Prefer tryFold over folding Results by hand",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
