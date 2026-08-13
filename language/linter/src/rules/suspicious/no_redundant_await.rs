use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow awaiting values that add no suspension.
    pub NO_REDUNDANT_AWAIT {
        id: "no-redundant-await",
        summary: "Disallow awaiting values that add no suspension",
        category: Suspicious,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
