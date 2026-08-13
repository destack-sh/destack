use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow draining a collection only to collect the same elements again.
    pub DRAIN_COLLECT {
        id: "drain-collect",
        summary: "Disallow draining a collection only to collect the same elements again",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
