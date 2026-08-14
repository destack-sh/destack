use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer unstable sorting when equal-element order is irrelevant.
    pub PREFER_UNSTABLE_SORT {
        id: "prefer-unstable-sort",
        summary: "Prefer unstable sorting when equal-element order is irrelevant",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
