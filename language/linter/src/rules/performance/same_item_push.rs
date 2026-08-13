use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer bulk initialization over repeatedly appending the same value.
    pub SAME_ITEM_PUSH {
        id: "same-item-push",
        summary: "Prefer bulk initialization over repeatedly appending the same value",
        category: Performance,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
