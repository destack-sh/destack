use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow updates derived from values read before suspension.
    pub STALE_UPDATE_ACROSS_SUSPENSION {
        id: "stale-update-across-suspension",
        summary: "Disallow updates derived from values read before suspension",
        category: Correctness,
        level: Error,
        fixable: None,
        check: MirModule,
    }
}
