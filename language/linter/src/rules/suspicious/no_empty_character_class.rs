use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow regex character classes that cannot match.
    pub NO_EMPTY_CHARACTER_CLASS {
        id: "no-empty-character-class",
        summary: "Disallow regex character classes that cannot match",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
