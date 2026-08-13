use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow compound assignments that repeat the assigned place.
    pub MISREFACTORED_ASSIGN_OP {
        id: "misrefactored-assign-op",
        summary: "Disallow compound assignments that repeat the assigned place",
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
