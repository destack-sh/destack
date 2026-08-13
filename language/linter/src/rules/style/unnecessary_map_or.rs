use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer a direct predicate or fallback operation over mapOr.
    pub UNNECESSARY_MAP_OR {
        id: "unnecessary-map-or",
        summary: "Prefer a direct predicate or fallback operation over mapOr",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
