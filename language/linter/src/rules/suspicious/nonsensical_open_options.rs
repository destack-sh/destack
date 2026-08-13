use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow contradictory or ineffective file open options.
    pub NONSENSICAL_OPEN_OPTIONS {
        id: "nonsensical-open-options",
        summary: "Disallow contradictory or ineffective file open options",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
