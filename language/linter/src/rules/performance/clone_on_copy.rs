use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow cloning values whose checked type is Copy.
    pub CLONE_ON_COPY {
        id: "clone-on-copy",
        summary: "Disallow cloning values whose checked type is Copy",
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
