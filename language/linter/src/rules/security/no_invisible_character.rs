use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow invisible and misleading characters in source text.
    pub NO_INVISIBLE_CHARACTER {
        id: "no-invisible-character",
        summary: "Disallow invisible and misleading characters in source text",
        category: Security,
        level: Error,
        fixable: None,
        check: DirModule,
    }
}
