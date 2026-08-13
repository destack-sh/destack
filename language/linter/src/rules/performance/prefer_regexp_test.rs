use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer RegExp.test when only match existence is observed.
    pub PREFER_REGEXP_TEST {
        id: "prefer-regexp-test",
        summary: "Prefer RegExp.test when only match existence is observed",
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
