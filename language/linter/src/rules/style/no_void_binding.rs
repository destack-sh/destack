use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow binding void values.
    pub NO_VOID_BINDING {
        id: "no-void-binding",
        summary: "Disallow binding void values",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
