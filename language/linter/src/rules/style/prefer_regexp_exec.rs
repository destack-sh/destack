use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer RegExp.exec when regular-expression match details are consumed.
    pub PREFER_REGEXP_EXEC {
        id: "prefer-regexp-exec",
        summary: "Prefer RegExp.exec when regular-expression match details are consumed",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
