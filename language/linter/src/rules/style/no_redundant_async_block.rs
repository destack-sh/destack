use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow async wrappers around a single awaited expression.
    pub NO_REDUNDANT_ASYNC_BLOCK {
        id: "no-redundant-async-block",
        summary: "Disallow async wrappers around a single awaited expression",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
