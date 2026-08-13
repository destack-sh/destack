use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer for-of when a loop only indexes one iterable.
    pub PREFER_FOR_OF {
        id: "prefer-for-of",
        summary: "Prefer for-of when a loop only indexes one iterable",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
