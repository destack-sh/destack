use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer expression initialization over staged assignment.
    pub PREFER_EXPRESSION_INITIALIZATION {
        id: "prefer-expression-initialization",
        summary: "Prefer expression initialization over staged assignment",
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
