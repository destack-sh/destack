use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer startsWith or endsWith over equivalent string comparisons.
    pub PREFER_STRING_STARTS_ENDS_WITH {
        id: "prefer-string-starts-ends-with",
        summary: "Prefer startsWith or endsWith over equivalent string comparisons",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
