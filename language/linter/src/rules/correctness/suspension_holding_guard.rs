use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow suspension while holding a guard or exclusive resource.
    pub SUSPENSION_HOLDING_GUARD {
        id: "suspension-holding-guard",
        summary: "Disallow suspension while holding a guard or exclusive resource",
        category: Correctness,
        level: Error,
        fixable: None,
        check: MirModule,
    }
}
