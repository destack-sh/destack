use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Enforce filename case style.
    pub FILENAME_CASE {
        id: "filename-case",
        summary: "Enforce filename case style",
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
