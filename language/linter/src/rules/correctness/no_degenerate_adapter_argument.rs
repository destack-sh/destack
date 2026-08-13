use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow constant arguments that make an operation a no-op or certain failure.
    pub NO_DEGENERATE_ADAPTER_ARGUMENT {
        id: "no-degenerate-adapter-argument",
        summary: "Disallow constant arguments that make an operation a no-op or certain failure",
        category: Correctness,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
