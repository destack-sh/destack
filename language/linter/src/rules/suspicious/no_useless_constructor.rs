use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow constructors that only repeat implicit construction behavior.
    pub NO_USELESS_CONSTRUCTOR {
        id: "no-useless-constructor",
        summary: "Disallow constructors that only repeat implicit construction behavior",
        category: Suspicious,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
