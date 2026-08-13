use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow wildcard arms that discard the error value.
    pub NO_WILDCARD_ERROR_ARM {
        id: "no-wildcard-error-arm",
        summary: "Disallow wildcard arms that discard the error value",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
