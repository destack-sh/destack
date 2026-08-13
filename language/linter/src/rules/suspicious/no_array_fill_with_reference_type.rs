use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow Array.fill values whose type has reference identity.
    pub NO_ARRAY_FILL_WITH_REFERENCE_TYPE {
        id: "no-array-fill-with-reference-type",
        summary: "Disallow Array.fill values whose type has reference identity",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
