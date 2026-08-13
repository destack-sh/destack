use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow escape sequences that do not change the parsed value.
    pub NO_USELESS_ESCAPE {
        id: "no-useless-escape",
        summary: "Disallow escape sequences that do not change the parsed value",
        category: Suspicious,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
