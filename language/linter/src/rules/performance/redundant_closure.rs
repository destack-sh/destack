use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow closures that only forward their arguments to another callable.
    pub REDUNDANT_CLOSURE {
        id: "redundant-closure",
        summary: "Disallow closures that only forward their arguments to another callable",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
