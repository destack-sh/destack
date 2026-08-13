use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer nesting or-patterns inside their shared constructor.
    pub PREFER_NESTED_OR_PATTERN {
        id: "prefer-nested-or-pattern",
        summary: "Prefer nesting or-patterns inside their shared constructor",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
