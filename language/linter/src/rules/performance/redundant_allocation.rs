use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow allocations that add no required ownership or representation.
    pub REDUNDANT_ALLOCATION {
        id: "redundant-allocation",
        summary: "Disallow allocations that add no required ownership or representation",
        category: Performance,
        level: Warning,
        fixable: None,
        check: MirModule,
    }
}
