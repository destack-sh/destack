use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Move identical branch prefixes or suffixes outside the conditional.
    pub BRANCHES_SHARING_CODE {
        id: "branches-sharing-code",
        summary: "Move identical branch prefixes or suffixes outside the conditional",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
