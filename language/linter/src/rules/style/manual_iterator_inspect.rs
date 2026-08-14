use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer Iterator.inspect when mapping observes and returns each value.
    pub MANUAL_ITERATOR_INSPECT {
        id: "manual-iterator-inspect",
        summary: "Prefer Iterator.inspect when mapping observes and returns each value",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
