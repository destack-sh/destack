use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer keys or values over projecting entries.
    pub PREFER_MAP_KEYS_VALUES {
        id: "prefer-map-keys-values",
        summary: "Prefer keys or values over projecting entries",
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
