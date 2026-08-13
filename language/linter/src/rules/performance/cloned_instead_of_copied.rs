use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer copying iterator elements whose type is Copy.
    pub CLONED_INSTEAD_OF_COPIED {
        id: "cloned-instead-of-copied",
        summary: "Prefer copying iterator elements whose type is Copy",
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
