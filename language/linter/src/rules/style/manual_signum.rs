use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer signum over equivalent sign branching.
    pub MANUAL_SIGNUM {
        id: "manual-signum",
        summary: "Prefer signum over equivalent sign branching",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
