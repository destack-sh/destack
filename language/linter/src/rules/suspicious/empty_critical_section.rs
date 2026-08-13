use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow locks held over nothing.
    pub EMPTY_CRITICAL_SECTION {
        id: "empty-critical-section",
        summary: "Disallow locks held over nothing",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: MirModule,
    }
}
