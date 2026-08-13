use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow Promise.resolve calls that preserve the same Promise.
    pub NO_USELESS_PROMISE_RESOLVE {
        id: "no-useless-promise-resolve",
        summary: "Disallow Promise.resolve calls that preserve the same Promise",
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
