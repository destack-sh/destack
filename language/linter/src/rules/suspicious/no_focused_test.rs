use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow focused tests.
    pub NO_FOCUSED_TEST {
        id: "no-focused-test",
        summary: "Disallow focused tests",
        category: Suspicious,
        level: Error,
        fixable: Automatic,
        check: DirModule,
    }
}
