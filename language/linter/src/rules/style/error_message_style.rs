use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Enforce error message casing and punctuation.
    pub ERROR_MESSAGE_STYLE {
        id: "error-message-style",
        summary: "Enforce error message casing and punctuation",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
