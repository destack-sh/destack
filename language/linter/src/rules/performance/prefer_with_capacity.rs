use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer construction with capacity over immediate reservation.
    pub PREFER_WITH_CAPACITY {
        id: "prefer-with-capacity",
        summary: "Prefer construction with capacity over immediate reservation",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
