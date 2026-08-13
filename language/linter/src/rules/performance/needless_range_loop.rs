use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer direct iteration over indexing through a collection range.
    pub NEEDLESS_RANGE_LOOP {
        id: "needless-range-loop",
        summary: "Prefer direct iteration over indexing through a collection range",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
