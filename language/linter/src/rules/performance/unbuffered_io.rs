use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow byte-at-a-time reads on unbuffered sources.
    pub UNBUFFERED_IO {
        id: "unbuffered-io",
        summary: "Disallow byte-at-a-time reads on unbuffered sources",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
