use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Require must-use marking for public functions whose value is their only effect.
    pub MUST_USE_CANDIDATE {
        id: "must-use-candidate",
        summary: "Require must-use marking for public functions whose value is their only effect",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: MirProgram,
    }
}
