use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer optional types over Results with unit errors.
    pub NO_UNIT_ERROR_RESULT {
        id: "no-unit-error-result",
        summary: "Prefer optional types over Results with unit errors",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
