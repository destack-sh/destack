use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow operator implementations built on a different operator.
    pub SUSPICIOUS_OPERATOR_IMPLEMENTATION {
        id: "suspicious-operator-implementation",
        summary: "Disallow operator implementations built on a different operator",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
