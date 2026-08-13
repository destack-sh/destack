use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer lazy fallbacks when eager arguments perform avoidable work.
    pub PREFER_LAZY_ARGUMENT {
        id: "prefer-lazy-argument",
        summary: "Prefer lazy fallbacks when eager arguments perform avoidable work",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
