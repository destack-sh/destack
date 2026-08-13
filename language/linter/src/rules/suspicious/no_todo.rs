use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow calls to the canonical todo function.
    pub NO_TODO {
        id: "no-todo",
        summary: "Disallow calls to the canonical todo function",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
