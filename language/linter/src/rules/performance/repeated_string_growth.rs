use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow repeated string growth that causes cumulative copying.
    pub REPEATED_STRING_GROWTH {
        id: "repeated-string-growth",
        summary: "Disallow repeated string growth that causes cumulative copying",
        category: Performance,
        level: Warning,
        fixable: None,
        check: MirModule,
    }
}
