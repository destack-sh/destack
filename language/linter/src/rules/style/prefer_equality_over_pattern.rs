use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer equality over matching a complete literal pattern.
    pub PREFER_EQUALITY_OVER_PATTERN {
        id: "prefer-equality-over-pattern",
        summary: "Prefer equality over matching a complete literal pattern",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
