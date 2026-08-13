use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer slice parameters over borrowed owned containers.
    pub PREFER_SLICE_PARAMETER {
        id: "prefer-slice-parameter",
        summary: "Prefer slice parameters over borrowed owned containers",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
