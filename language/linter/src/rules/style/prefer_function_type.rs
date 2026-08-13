use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer function types over object types containing only a call signature.
    pub PREFER_FUNCTION_TYPE {
        id: "prefer-function-type",
        summary: "Prefer function types over object types containing only a call signature",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
