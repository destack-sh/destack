use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow acquiring a lock while its guard is still live.
    pub NO_DOUBLE_LOCK {
        id: "no-double-lock",
        summary: "Disallow acquiring a lock while its guard is still live",
        category: Correctness,
        level: Error,
        fixable: None,
        check: MirModule,
    }
}
