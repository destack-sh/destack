use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow unwrap after control flow already proves the present case.
    pub UNNECESSARY_UNWRAP {
        id: "unnecessary-unwrap",
        summary: "Disallow unwrap after control flow already proves the present case",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
