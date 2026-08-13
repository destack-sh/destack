use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer async functions over returning an async closure immediately.
    pub MANUAL_ASYNC_FUNCTION {
        id: "manual-async-function",
        summary: "Prefer async functions over returning an async closure immediately",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
