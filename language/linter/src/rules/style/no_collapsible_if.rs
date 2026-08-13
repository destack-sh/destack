use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Suggest merging nested if statements.
    pub NO_COLLAPSIBLE_IF {
        id: "no-collapsible-if",
        summary: "Suggest merging nested if statements",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
