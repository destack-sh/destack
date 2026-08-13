use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer stored length over counting an iterator.
    pub PREFER_LENGTH_OVER_COUNT {
        id: "prefer-length-over-count",
        summary: "Prefer stored length over counting an iterator",
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
