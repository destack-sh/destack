use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow binding unit values.
    pub NO_UNIT_BINDING {
        id: "no-unit-binding",
        summary: "Disallow binding unit values",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
