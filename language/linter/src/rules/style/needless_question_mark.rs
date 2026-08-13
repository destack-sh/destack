use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow propagation immediately wrapped in the same result form.
    pub NEEDLESS_QUESTION_MARK {
        id: "needless-question-mark",
        summary: "Disallow propagation immediately wrapped in the same result form",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
