use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer sets over maps whose values are void.
    pub PREFER_SET_OVER_VOID_MAP {
        id: "prefer-set-over-void-map",
        summary: "Prefer sets over maps whose values are void",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
