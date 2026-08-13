use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer unwrapOr over equivalent pattern matching.
    pub MANUAL_UNWRAP_OR {
        id: "manual-unwrap-or",
        summary: "Prefer unwrapOr over equivalent pattern matching",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
