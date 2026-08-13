use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow functions longer than the canonical authored-line limit.
    pub LARGE_FUNCTION {
        id: "large-function",
        summary: "Disallow functions longer than the canonical authored-line limit",
        explanation: r#"
Large function bodies require readers to retain too much local state at once. Extract a coherent
operation when a function exceeds Destack's standard authored-line limit; blank lines and
comment-only lines do not count.
"#,
        example: {
            reported: r#"
function publish(article: Article): Result<Article, PublishError> {
    // validation, normalization, persistence, indexing, notification,
    // metrics, and audit logic continue beyond the standard limit
    return Result.ok(article);
}
"#,
            accepted: r#"
function publish(article: Article): Result<Article, PublishError> {
    const article = prepare(article)?;
    persist(article)?;
    announce(article)?;
    return Result.ok(article);
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
