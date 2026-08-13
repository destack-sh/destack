use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow key types whose hash or equality depends on mutable state.
    pub NO_MUTABLE_MAP_KEY {
        id: "no-mutable-map-key",
        summary: "Disallow key types whose hash or equality depends on mutable state",
        category: Correctness,
        level: Error,
        fixable: None,
        check: DirModule,
    }
}
