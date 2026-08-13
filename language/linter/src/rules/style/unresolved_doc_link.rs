use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow doc links that resolve to nothing.
    pub UNRESOLVED_DOC_LINK {
        id: "unresolved-doc-link",
        summary: "Disallow doc links that resolve to nothing",
        category: Style,
        level: Warning,
        fixable: None,
        check: DirProgram,
    }
}
