use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer tuple destructuring over indexed access.
    pub PREFER_TUPLE_DESTRUCTURE {
        id: "prefer-tuple-destructure",
        summary: "Prefer tuple destructuring over indexed access",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
