use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow ambiguous octal-adjacent escapes.
    pub NO_OCTAL_ESCAPE {
        id: "no-octal-escape",
        summary: "Disallow ambiguous octal-adjacent escapes",
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
