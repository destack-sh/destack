use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer unwrapOrDefault over equivalent pattern matching.
    pub MANUAL_UNWRAP_OR_DEFAULT {
        id: "manual-unwrap-or-default",
        summary: "Prefer unwrapOrDefault over equivalent pattern matching",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
