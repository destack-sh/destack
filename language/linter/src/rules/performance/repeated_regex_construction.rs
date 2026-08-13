use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow repeated construction of the same regular expression.
    pub REPEATED_REGEX_CONSTRUCTION {
        id: "repeated-regex-construction",
        summary: "Disallow repeated construction of the same regular expression",
        category: Performance,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
