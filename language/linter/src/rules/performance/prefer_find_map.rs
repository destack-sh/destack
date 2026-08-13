use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer findMap when only one transformed match is consumed.
    pub PREFER_FIND_MAP {
        id: "prefer-find-map",
        summary: "Prefer findMap when only one transformed match is consumed",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
