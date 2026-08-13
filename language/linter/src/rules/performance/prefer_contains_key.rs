use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer membership operations over lookups that discard their value.
    pub PREFER_CONTAINS_KEY {
        id: "prefer-contains-key",
        summary: "Prefer membership operations over lookups that discard their value",
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
