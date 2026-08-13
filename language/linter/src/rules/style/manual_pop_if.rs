use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer popIf over checking the last element before popping.
    pub MANUAL_POP_IF {
        id: "manual-pop-if",
        summary: "Prefer popIf over checking the last element before popping",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
