use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer nullish coalescing when only nullish values select the fallback.
    pub PREFER_NULLISH_COALESCING {
        id: "prefer-nullish-coalescing",
        summary: "Prefer nullish coalescing when only nullish values select the fallback",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
