use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer isSomeAnd over mapping to a defaulted predicate.
    pub MANUAL_IS_SOME_AND {
        id: "manual-is-some-and",
        summary: "Prefer isSomeAnd over mapping to a defaulted predicate",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
