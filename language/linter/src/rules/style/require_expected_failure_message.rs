use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Require expected messages on panic-expecting tests.
    pub REQUIRE_EXPECTED_FAILURE_MESSAGE {
        id: "require-expected-failure-message",
        summary: "Require expected messages on panic-expecting tests",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
