use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer nth over skip followed by next.
    pub PREFER_NTH {
        id: "prefer-nth",
        summary: "Prefer nth over skip followed by next",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
