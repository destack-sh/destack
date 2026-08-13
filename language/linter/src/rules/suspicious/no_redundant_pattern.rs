use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow patterns that bind nothing.
    pub NO_REDUNDANT_PATTERN {
        id: "no-redundant-pattern",
        summary: "Disallow patterns that bind nothing",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
