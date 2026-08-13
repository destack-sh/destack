use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer one map entry operation over repeated key lookups.
    pub PREFER_MAP_ENTRY {
        id: "prefer-map-entry",
        summary: "Prefer one map entry operation over repeated key lookups",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
