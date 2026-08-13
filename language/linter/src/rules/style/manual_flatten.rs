use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer flatten over manually discarding absent nested values.
    pub MANUAL_FLATTEN {
        id: "manual-flatten",
        summary: "Prefer flatten over manually discarding absent nested values",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
