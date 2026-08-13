use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Delay iterator cloning until after operations that discard elements.
    pub ITER_OVEREAGER_CLONED {
        id: "iter-overeager-cloned",
        summary: "Delay iterator cloning until after operations that discard elements",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
