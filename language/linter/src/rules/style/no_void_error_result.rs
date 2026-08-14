use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer optional types over Results whose error is void.
    pub NO_VOID_ERROR_RESULT {
        id: "no-void-error-result",
        summary: "Prefer optional types over Results whose error is void",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
