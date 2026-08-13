use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow accessors that return a different field than they name.
    pub MISNAMED_GETTER {
        id: "misnamed-getter",
        summary: "Disallow accessors that return a different field than they name",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
