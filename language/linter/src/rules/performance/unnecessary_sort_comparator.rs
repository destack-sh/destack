use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow comparators that reproduce the natural ordering.
    pub UNNECESSARY_SORT_COMPARATOR {
        id: "unnecessary-sort-comparator",
        summary: "Disallow comparators that reproduce the natural ordering",
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
