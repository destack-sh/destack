use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer Clone.cloneFrom when existing storage can be reused.
    pub PREFER_CLONE_FROM {
        id: "prefer-clone-from",
        summary: "Prefer Clone.cloneFrom when existing storage can be reused",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
