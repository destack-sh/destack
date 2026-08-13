use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer propagation over manually returning the absent or failed case.
    pub MANUAL_QUESTION_MARK {
        id: "manual-question-mark",
        summary: "Prefer propagation over manually returning the absent or failed case",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
