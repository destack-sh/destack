use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow filtering away every failure silently.
    pub NO_SILENTLY_DROPPED_ERROR {
        id: "no-silently-dropped-error",
        summary: "Disallow filtering away every failure silently",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
