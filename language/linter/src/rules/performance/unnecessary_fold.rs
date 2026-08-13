use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer a specialized iterator operation over an equivalent fold.
    pub UNNECESSARY_FOLD {
        id: "unnecessary-fold",
        summary: "Prefer a specialized iterator operation over an equivalent fold",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
