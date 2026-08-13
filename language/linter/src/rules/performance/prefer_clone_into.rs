use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer cloning into existing storage.
    pub PREFER_CLONE_INTO {
        id: "prefer-clone-into",
        summary: "Prefer cloning into existing storage",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
