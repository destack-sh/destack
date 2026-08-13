use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer unstable sorting for primitive elements.
    pub PREFER_UNSTABLE_SORT {
        id: "prefer-unstable-sort",
        summary: "Prefer unstable sorting for primitive elements",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
