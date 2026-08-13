use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Combine consecutive equivalent collection mutations into one call.
    pub PREFER_SINGLE_CALL {
        id: "prefer-single-call",
        summary: "Combine consecutive equivalent collection mutations into one call",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
