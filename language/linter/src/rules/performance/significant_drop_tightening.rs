use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Shorten the lifetime of values with significant destruction.
    pub SIGNIFICANT_DROP_TIGHTENING {
        id: "significant-drop-tightening",
        summary: "Shorten the lifetime of values with significant destruction",
        category: Performance,
        level: Warning,
        fixable: None,
        check: MirModule,
    }
}
