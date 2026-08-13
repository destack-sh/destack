use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer zip over pairing two presence checks.
    pub MANUAL_OPTION_ZIP {
        id: "manual-option-zip",
        summary: "Prefer zip over pairing two presence checks",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
