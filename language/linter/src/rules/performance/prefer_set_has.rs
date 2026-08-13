use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer set membership for repeated linear membership tests.
    pub PREFER_SET_HAS {
        id: "prefer-set-has",
        summary: "Prefer set membership for repeated linear membership tests",
        category: Performance,
        level: Warning,
        fixable: None,
        check: MirModule,
    }
}
