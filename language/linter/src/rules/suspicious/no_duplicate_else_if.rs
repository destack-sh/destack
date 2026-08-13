use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow duplicate or covered else-if conditions.
    pub NO_DUPLICATE_ELSE_IF {
        id: "no-duplicate-else-if",
        summary: "Disallow duplicate or covered else-if conditions",
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
