use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer flat over equivalent concatenation or spreading.
    pub PREFER_FLAT {
        id: "prefer-flat",
        summary: "Prefer flat over equivalent concatenation or spreading",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
