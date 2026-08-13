use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Merge nested matches whose inner pattern fits the outer arm.
    pub NO_COLLAPSIBLE_MATCH {
        id: "no-collapsible-match",
        summary: "Merge nested matches whose inner pattern fits the outer arm",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
