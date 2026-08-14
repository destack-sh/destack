use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer Result.tap or Result.tapErr when mapping observes and returns its value.
    pub MANUAL_RESULT_TAP {
        id: "manual-result-tap",
        summary: "Prefer Result tap methods when mapping observes and returns its value",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
