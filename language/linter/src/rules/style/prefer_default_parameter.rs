use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer default parameters over defaulting in the body.
    pub PREFER_DEFAULT_PARAMETER {
        id: "prefer-default-parameter",
        summary: "Prefer default parameters over defaulting in the body",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
