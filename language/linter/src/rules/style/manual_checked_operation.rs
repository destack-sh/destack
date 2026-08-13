use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer checked arithmetic over equivalent manual bounds checks.
    pub MANUAL_CHECKED_OPERATION {
        id: "manual-checked-operation",
        summary: "Prefer checked arithmetic over equivalent manual bounds checks",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
