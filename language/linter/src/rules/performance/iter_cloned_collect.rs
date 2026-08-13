use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer direct collection cloning over iterating, cloning, and collecting.
    pub ITER_CLONED_COLLECT {
        id: "iter-cloned-collect",
        summary: "Prefer direct collection cloning over iterating, cloning, and collecting",
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
