use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow materializing a collection consumed by one streaming operation.
    pub NEEDLESS_COLLECT {
        id: "needless-collect",
        summary: "Disallow materializing a collection consumed by one streaming operation",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
