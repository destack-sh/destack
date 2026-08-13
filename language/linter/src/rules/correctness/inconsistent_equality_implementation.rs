use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow inconsistent implementations across equality, ordering, and hashing protocols.
    pub INCONSISTENT_EQUALITY_IMPLEMENTATION {
        id: "inconsistent-equality-implementation",
        summary: "Disallow inconsistent implementations across equality, ordering, and hashing protocols",
        category: Correctness,
        level: Error,
        fixable: None,
        check: DirModule,
    }
}
