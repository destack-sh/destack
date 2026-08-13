use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow joining absolute paths onto a base.
    pub NO_ABSOLUTE_PATH_JOIN {
        id: "no-absolute-path-join",
        summary: "Disallow joining absolute paths onto a base",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
