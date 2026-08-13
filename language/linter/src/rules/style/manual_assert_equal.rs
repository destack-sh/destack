use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer equality assertions over asserting a comparison.
    pub MANUAL_ASSERT_EQUAL {
        id: "manual-assert-equal",
        summary: "Prefer equality assertions over asserting a comparison",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
