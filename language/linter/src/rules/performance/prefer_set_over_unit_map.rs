use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer sets over maps with unit values.
    pub PREFER_SET_OVER_UNIT_MAP {
        id: "prefer-set-over-unit-map",
        summary: "Prefer sets over maps with unit values",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
