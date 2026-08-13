use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow linear operations on repeated paths.
    pub LINEAR_OPERATION_IN_LOOP {
        id: "linear-operation-in-loop",
        summary: "Disallow linear operations on repeated paths",
        category: Performance,
        level: Warning,
        fixable: None,
        check: MirModule,
    }
}
