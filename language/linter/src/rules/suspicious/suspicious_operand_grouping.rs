use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow asymmetric operands in chains of similar comparisons.
    pub SUSPICIOUS_OPERAND_GROUPING {
        id: "suspicious-operand-grouping",
        summary: "Disallow asymmetric operands in chains of similar comparisons",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
