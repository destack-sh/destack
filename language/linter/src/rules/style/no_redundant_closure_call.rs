use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow immediately invoked closures.
    pub NO_REDUNDANT_CLOSURE_CALL {
        id: "no-redundant-closure-call",
        summary: "Disallow immediately invoked closures",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
