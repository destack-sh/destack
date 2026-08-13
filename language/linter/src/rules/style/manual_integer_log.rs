use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer integer logarithms over float log arithmetic.
    pub MANUAL_INTEGER_LOG {
        id: "manual-integer-log",
        summary: "Prefer integer logarithms over float log arithmetic",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
