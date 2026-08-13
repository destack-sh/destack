use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow diverging subexpressions inside larger expressions.
    pub NO_DIVERGING_SUBEXPRESSION {
        id: "no-diverging-subexpression",
        summary: "Disallow diverging subexpressions inside larger expressions",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
