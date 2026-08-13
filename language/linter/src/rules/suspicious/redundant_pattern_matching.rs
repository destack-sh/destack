use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer direct result predicates over pattern matching used only as a test.
    pub REDUNDANT_PATTERN_MATCHING {
        id: "redundant-pattern-matching",
        summary: "Prefer direct result predicates over pattern matching used only as a test",
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
