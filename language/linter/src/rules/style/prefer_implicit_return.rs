use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer implicit return for arrow functions.
    pub PREFER_IMPLICIT_RETURN {
        id: "prefer-implicit-return",
        summary: "Prefer implicit return for arrow functions",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
