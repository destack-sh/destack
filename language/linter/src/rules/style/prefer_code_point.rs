use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer code-point operations when text may contain supplementary characters.
    pub PREFER_CODE_POINT {
        id: "prefer-code-point",
        summary: "Prefer code-point operations when text may contain supplementary characters",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
