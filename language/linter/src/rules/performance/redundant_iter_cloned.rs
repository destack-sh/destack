use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow cloning iterator elements that are only borrowed afterward.
    pub REDUNDANT_ITER_CLONED {
        id: "redundant-iter-cloned",
        summary: "Disallow cloning iterator elements that are only borrowed afterward",
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
