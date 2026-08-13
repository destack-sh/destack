use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow adapters whose added capability is never used.
    pub UNUSED_PEEKABLE {
        id: "unused-peekable",
        summary: "Disallow adapters whose added capability is never used",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
