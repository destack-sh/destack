use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow multi-character trim sets where a prefix strip was intended.
    pub NO_MISLEADING_TRIM_SET {
        id: "no-misleading-trim-set",
        summary: "Disallow multi-character trim sets where a prefix strip was intended",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
