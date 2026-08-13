use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer string slice over legacy substring operations.
    pub PREFER_STRING_SLICE {
        id: "prefer-string-slice",
        summary: "Prefer string slice over legacy substring operations",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
