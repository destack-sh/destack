use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow map callbacks used only for their effects.
    pub NO_SIDE_EFFECTING_MAP {
        id: "no-side-effecting-map",
        summary: "Disallow map callbacks used only for their effects",
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
