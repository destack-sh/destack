use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer join over collecting formatted pieces.
    pub PREFER_JOIN_OVER_STRING_COLLECT {
        id: "prefer-join-over-string-collect",
        summary: "Prefer join over collecting formatted pieces",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
