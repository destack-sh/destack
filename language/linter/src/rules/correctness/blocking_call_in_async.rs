use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow resolved blocking operations in asynchronous execution.
    pub BLOCKING_CALL_IN_ASYNC {
        id: "blocking-call-in-async",
        summary: "Disallow resolved blocking operations in asynchronous execution",
        category: Correctness,
        level: Error,
        fixable: None,
        check: MirModule,
    }
}
