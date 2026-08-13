use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer splitOnce over splitting for one separator.
    pub MANUAL_SPLIT_ONCE {
        id: "manual-split-once",
        summary: "Prefer splitOnce over splitting for one separator",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
