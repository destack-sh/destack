use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow exclusive locks on paths that only read.
    pub UNNECESSARY_EXCLUSIVE_LOCK {
        id: "unnecessary-exclusive-lock",
        summary: "Disallow exclusive locks on paths that only read",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: MirModule,
    }
}
