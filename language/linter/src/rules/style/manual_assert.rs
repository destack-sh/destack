use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer assertions over conditional panics.
    pub MANUAL_ASSERT {
        id: "manual-assert",
        summary: "Prefer assertions over conditional panics",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
