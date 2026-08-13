use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Require isEmpty alongside a length accessor.
    pub REQUIRE_IS_EMPTY_WITH_LENGTH {
        id: "require-is-empty-with-length",
        summary: "Require isEmpty alongside a length accessor",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
