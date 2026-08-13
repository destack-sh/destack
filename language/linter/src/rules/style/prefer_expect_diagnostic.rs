use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer diagnostic expectations for local suppressions.
    pub PREFER_EXPECT_DIAGNOSTIC {
        id: "prefer-expect-diagnostic",
        summary: "Prefer diagnostic expectations for local suppressions",
        explanation: r#"
A local `@allow` remains silently valid after the suppressed diagnostic disappears. Use `@expect`
when a specific construct intentionally produces a diagnostic so removal of the diagnostic also
removes the stale suppression.
"#,
        example: {
            reported: r#"
@allow("constant-condition", { reason: "required sentinel branch" })
function ready(): boolean {
    if (true) {
        return true;
    }

    return false;
}
"#,
            accepted: r#"
@expect("constant-condition", { reason: "required sentinel branch" })
function ready(): boolean {
    if (true) {
        return true;
    }

    return false;
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
