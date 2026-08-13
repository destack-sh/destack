use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Require the canonical extension declaration form for its visibility.
    pub CONSISTENT_EXTENSION_STYLE {
        id: "consistent-extension-style",
        summary: "Require the canonical extension declaration form for its visibility",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
