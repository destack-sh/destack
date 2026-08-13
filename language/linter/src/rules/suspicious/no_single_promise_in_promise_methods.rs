use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow aggregate Promise operations over one Promise.
    pub NO_SINGLE_PROMISE_IN_PROMISE_METHODS {
        id: "no-single-promise-in-promise-methods",
        summary: "Disallow aggregate Promise operations over one Promise",
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
