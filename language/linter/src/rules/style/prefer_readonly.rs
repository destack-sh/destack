use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Require readonly for fields never mutated after initialization.
    pub PREFER_READONLY {
        id: "prefer-readonly",
        summary: "Require readonly for fields never mutated after initialization",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
