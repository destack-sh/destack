use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow length checks duplicated by the guarded operation.
    pub NO_USELESS_LENGTH_CHECK {
        id: "no-useless-length-check",
        summary: "Disallow length checks duplicated by the guarded operation",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
