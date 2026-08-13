use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow skipped tests without a reason.
    pub NO_SKIPPED_TEST {
        id: "no-skipped-test",
        summary: "Disallow skipped tests without a reason",
        category: Suspicious,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
