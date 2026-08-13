use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow reading and mutating the same place within one larger expression.
    pub MIXED_READ_WRITE_EXPRESSION {
        id: "mixed-read-write-expression",
        summary: "Disallow reading and mutating the same place within one larger expression",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
