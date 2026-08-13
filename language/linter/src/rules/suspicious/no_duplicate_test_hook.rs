use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow repeated lifecycle hooks in one suite.
    pub NO_DUPLICATE_TEST_HOOK {
        id: "no-duplicate-test-hook",
        summary: "Disallow repeated lifecycle hooks in one suite",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
