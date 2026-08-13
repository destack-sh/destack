use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow borrowing small Copy values where copying is cheaper.
    pub NEEDLESS_BORROW_OF_COPY {
        id: "needless-borrow-of-copy",
        summary: "Disallow borrowing small Copy values where copying is cheaper",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
