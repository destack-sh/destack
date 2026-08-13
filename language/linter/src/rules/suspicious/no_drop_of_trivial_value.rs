use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow dropping values without destructors.
    pub NO_DROP_OF_TRIVIAL_VALUE {
        id: "no-drop-of-trivial-value",
        summary: "Disallow dropping values without destructors",
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
