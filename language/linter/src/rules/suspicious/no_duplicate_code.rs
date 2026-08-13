use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow substantial alpha-equivalent checked code.
    pub NO_DUPLICATE_CODE {
        id: "no-duplicate-code",
        summary: "Disallow substantial alpha-equivalent checked code",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirProgram,
    }
}
