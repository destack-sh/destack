use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow wildcard arms that hide later enum variants.
    pub WILDCARD_ENUM_MATCH_ARM {
        id: "wildcard-enum-match-arm",
        summary: "Disallow wildcard arms that hide later enum variants",
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
