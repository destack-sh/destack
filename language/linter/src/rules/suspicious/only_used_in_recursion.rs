use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow values that only contribute to recursive calls of their own function.
    pub ONLY_USED_IN_RECURSION {
        id: "only-used-in-recursion",
        summary: "Disallow values that only contribute to recursive calls of their own function",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
