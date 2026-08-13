use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer String.raw for strings dominated by escape sequences.
    pub PREFER_STRING_RAW {
        id: "prefer-string-raw",
        summary: "Prefer String.raw for strings dominated by escape sequences",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
