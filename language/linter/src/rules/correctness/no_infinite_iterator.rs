use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow consuming provably unbounded iterators.
    pub NO_INFINITE_ITERATOR {
        id: "no-infinite-iterator",
        summary: "Disallow consuming provably unbounded iterators",
        category: Correctness,
        level: Error,
        fixable: None,
        check: DirModule,
    }
}
