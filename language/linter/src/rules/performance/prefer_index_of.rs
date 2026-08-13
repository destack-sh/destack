use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer indexOf when a predicate only compares one value.
    pub PREFER_INDEX_OF {
        id: "prefer-index-of",
        summary: "Prefer indexOf when a predicate only compares one value",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
