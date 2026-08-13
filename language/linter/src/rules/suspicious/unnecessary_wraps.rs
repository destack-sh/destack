use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow functions that always return the same optional or result case.
    pub UNNECESSARY_WRAPS {
        id: "unnecessary-wraps",
        summary: "Disallow functions that always return the same optional or result case",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
