use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow functions that require excessive stack storage.
    pub LARGE_STACK_FRAME {
        id: "large-stack-frame",
        summary: "Disallow functions that require excessive stack storage",
        category: Performance,
        level: Warning,
        fixable: None,
        check: MirModule,
    }
}
