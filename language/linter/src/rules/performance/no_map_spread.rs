use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow copying each mapped value solely to extend it.
    pub NO_MAP_SPREAD {
        id: "no-map-spread",
        summary: "Disallow copying each mapped value solely to extend it",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
