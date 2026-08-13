use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer for-of over repeatedly requesting the next iterator value.
    pub WHILE_LET_ON_ITERATOR {
        id: "while-let-on-iterator",
        summary: "Prefer for-of over repeatedly requesting the next iterator value",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
