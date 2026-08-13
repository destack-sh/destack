use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow allocating conversions performed only to compare.
    pub NO_ALLOCATION_FOR_COMPARISON {
        id: "no-allocation-for-comparison",
        summary: "Disallow allocating conversions performed only to compare",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
