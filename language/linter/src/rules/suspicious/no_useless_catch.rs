use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow catch clauses that only propagate the caught failure.
    pub NO_USELESS_CATCH {
        id: "no-useless-catch",
        summary: "Disallow catch clauses that only propagate the caught failure",
        category: Suspicious,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
