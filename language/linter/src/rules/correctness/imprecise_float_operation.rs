use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer numerically stable floating-point operations.
    pub IMPRECISE_FLOAT_OPERATION {
        id: "imprecise-float-operation",
        summary: "Prefer numerically stable floating-point operations",
        category: Correctness,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
