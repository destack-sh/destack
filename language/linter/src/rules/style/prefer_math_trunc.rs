use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer trunc over an equivalent integer conversion.
    pub PREFER_MATH_TRUNC {
        id: "prefer-math-trunc",
        summary: "Prefer trunc over an equivalent integer conversion",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
