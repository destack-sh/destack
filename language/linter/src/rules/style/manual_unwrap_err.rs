use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer unwrapErr over projecting the failed case manually.
    pub MANUAL_UNWRAP_ERR {
        id: "manual-unwrap-err",
        summary: "Prefer unwrapErr over projecting the failed case manually",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
