use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow constant regex sources that fail to compile.
    pub NO_INVALID_REGEXP {
        id: "no-invalid-regexp",
        summary: "Disallow constant regex sources that fail to compile",
        category: Correctness,
        level: Error,
        fixable: None,
        check: DirModule,
    }
}
