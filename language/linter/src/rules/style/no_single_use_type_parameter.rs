use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow type parameters used exactly once.
    pub NO_SINGLE_USE_TYPE_PARAMETER {
        id: "no-single-use-type-parameter",
        summary: "Disallow type parameters used exactly once",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
