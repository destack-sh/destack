use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow match arms with identical checked bodies.
    pub NO_DUPLICATE_MATCH_ARMS {
        id: "no-duplicate-match-arms",
        summary: "Disallow match arms with identical checked bodies",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
