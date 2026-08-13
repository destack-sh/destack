use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Simplify boolean expressions with redundant or contradictory terms.
    pub NONMINIMAL_BOOL {
        id: "nonminimal-bool",
        summary: "Simplify boolean expressions with redundant or contradictory terms",
        category: Suspicious,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
