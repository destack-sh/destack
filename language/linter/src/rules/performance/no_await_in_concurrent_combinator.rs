use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow awaiting inside concurrent combinator arguments.
    pub NO_AWAIT_IN_CONCURRENT_COMBINATOR {
        id: "no-await-in-concurrent-combinator",
        summary: "Disallow awaiting inside concurrent combinator arguments",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
