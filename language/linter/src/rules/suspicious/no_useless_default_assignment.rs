use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow defaults that cannot be selected by the checked input type.
    pub NO_USELESS_DEFAULT_ASSIGNMENT {
        id: "no-useless-default-assignment",
        summary: "Disallow defaults that cannot be selected by the checked input type",
        category: Suspicious,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
