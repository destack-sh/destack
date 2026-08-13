use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow excessive coroutine state retained across suspension.
    pub LARGE_COROUTINE_STATE {
        id: "large-coroutine-state",
        summary: "Disallow excessive coroutine state retained across suspension",
        category: Performance,
        level: Warning,
        fixable: None,
        check: MirModule,
    }
}
