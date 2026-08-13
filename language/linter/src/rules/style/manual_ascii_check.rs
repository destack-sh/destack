use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer character predicates over range comparisons.
    pub MANUAL_ASCII_CHECK {
        id: "manual-ascii-check",
        summary: "Prefer character predicates over range comparisons",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
