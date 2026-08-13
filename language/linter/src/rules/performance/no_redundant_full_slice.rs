use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow full-range slicing where a borrow suffices.
    pub NO_REDUNDANT_FULL_SLICE {
        id: "no-redundant-full-slice",
        summary: "Disallow full-range slicing where a borrow suffices",
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
