use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow classes containing only static members.
    pub NO_STATIC_ONLY_CLASS {
        id: "no-static-only-class",
        summary: "Disallow classes containing only static members",
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
