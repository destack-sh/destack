use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer repeat over concatenating one string in a loop.
    pub MANUAL_REPEAT_STRING {
        id: "manual-repeat-string",
        summary: "Prefer repeat over concatenating one string in a loop",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
