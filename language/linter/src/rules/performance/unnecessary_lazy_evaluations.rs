use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer eager fallbacks when deferred evaluation cannot avoid work.
    pub UNNECESSARY_LAZY_EVALUATIONS {
        id: "unnecessary-lazy-evaluations",
        summary: "Prefer eager fallbacks when deferred evaluation cannot avoid work",
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
