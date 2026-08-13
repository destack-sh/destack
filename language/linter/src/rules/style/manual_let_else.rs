use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer let-else over an equivalent match or conditional.
    pub MANUAL_LET_ELSE {
        id: "manual-let-else",
        summary: "Prefer let-else over an equivalent match or conditional",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
