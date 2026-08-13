use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer faster floating-point operations when their precision is sufficient.
    pub SUBOPTIMAL_FLOAT_OPERATION {
        id: "suboptimal-float-operation",
        summary: "Prefer faster floating-point operations when their precision is sufficient",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
