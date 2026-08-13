use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer take over copying a value and clearing its place.
    pub MANUAL_TAKE {
        id: "manual-take",
        summary: "Prefer take over copying a value and clearing its place",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
