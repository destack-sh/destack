use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer prefix or suffix stripping over checking and slicing.
    pub MANUAL_STRIP {
        id: "manual-strip",
        summary: "Prefer prefix or suffix stripping over checking and slicing",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
