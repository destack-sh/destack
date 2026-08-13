use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer clear over draining into nothing.
    pub PREFER_CLEAR {
        id: "prefer-clear",
        summary: "Prefer clear over draining into nothing",
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
