use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow variants that disproportionately enlarge an inline union.
    pub LARGE_VARIANT {
        id: "large-variant",
        summary: "Disallow variants that disproportionately enlarge an inline union",
        category: Performance,
        level: Warning,
        fixable: None,
        check: MirModule,
    }
}
