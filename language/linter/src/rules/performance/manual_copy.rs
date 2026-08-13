use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Replace element-by-element copy loops with a bulk copy operation.
    pub MANUAL_COPY {
        id: "manual-copy",
        summary: "Replace element-by-element copy loops with a bulk copy operation",
        category: Performance,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
