use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer derives over hand-written equivalent implementations.
    pub PREFER_DERIVED_EXTENSION {
        id: "prefer-derived-extension",
        summary: "Prefer derives over hand-written equivalent implementations",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
