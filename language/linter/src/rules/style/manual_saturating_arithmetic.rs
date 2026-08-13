use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer saturating arithmetic over equivalent manual bounds logic.
    pub MANUAL_SATURATING_ARITHMETIC {
        id: "manual-saturating-arithmetic",
        summary: "Prefer saturating arithmetic over equivalent manual bounds logic",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
