use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow mutation inside debug-only assertions.
    pub NO_SIDE_EFFECT_IN_DEBUG_ASSERT {
        id: "no-side-effect-in-debug-assert",
        summary: "Disallow mutation inside debug-only assertions",
        category: Correctness,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
