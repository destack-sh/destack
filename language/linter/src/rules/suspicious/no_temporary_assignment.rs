use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow assigning into values that are immediately discarded.
    pub NO_TEMPORARY_ASSIGNMENT {
        id: "no-temporary-assignment",
        summary: "Disallow assigning into values that are immediately discarded",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
