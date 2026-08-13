use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer slice patterns over indexing after a length guard.
    pub PREFER_SLICE_PATTERN {
        id: "prefer-slice-pattern",
        summary: "Prefer slice patterns over indexing after a length guard",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
