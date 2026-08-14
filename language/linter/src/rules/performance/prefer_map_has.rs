use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer Map.has over lookups that discard the value.
    pub PREFER_MAP_HAS {
        id: "prefer-map-has",
        summary: "Prefer Map.has over lookups that discard the value",
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
