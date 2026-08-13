use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow misleading regex character classes.
    pub NO_MISLEADING_CHARACTER_CLASS {
        id: "no-misleading-character-class",
        summary: "Disallow misleading regex character classes",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
