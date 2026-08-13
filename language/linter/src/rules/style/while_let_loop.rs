use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer a pattern loop over an equivalent loop and match.
    pub WHILE_LET_LOOP {
        id: "while-let-loop",
        summary: "Prefer a pattern loop over an equivalent loop and match",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
