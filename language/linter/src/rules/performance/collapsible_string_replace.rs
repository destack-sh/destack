use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Combine chained replacements sharing one replacement value.
    pub COLLAPSIBLE_STRING_REPLACE {
        id: "collapsible-string-replace",
        summary: "Combine chained replacements sharing one replacement value",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
