use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow enum members sharing one explicit value.
    pub NO_DUPLICATE_ENUM_VALUE {
        id: "no-duplicate-enum-value",
        summary: "Disallow enum members sharing one explicit value",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
