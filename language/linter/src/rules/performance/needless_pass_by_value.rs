use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow passing parameters by value when they are never consumed or mutated.
    pub NEEDLESS_PASS_BY_VALUE {
        id: "needless-pass-by-value",
        summary: "Disallow passing parameters by value when they are never consumed or mutated",
        category: Performance,
        level: Warning,
        fixable: None,
        check: MirModule,
    }
}
