use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer match over complex if-else-if or switch statements.
    pub PREFER_MATCH {
        id: "prefer-match",
        summary: "Prefer match over complex if-else-if or switch statements",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
