use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer tuple assignment for swaps.
    pub PREFER_TUPLE_SWAP {
        id: "prefer-tuple-swap",
        summary: "Prefer tuple assignment for swaps",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
