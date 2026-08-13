use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer multiplication over small constant powers.
    pub PREFER_MULTIPLICATION_OVER_POWER {
        id: "prefer-multiplication-over-power",
        summary: "Prefer multiplication over small constant powers",
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
