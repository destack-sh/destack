use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer Result projection methods over equivalent pattern matching.
    pub MANUAL_OK_ERR {
        id: "manual-ok-err",
        summary: "Prefer Result projection methods over equivalent pattern matching",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
