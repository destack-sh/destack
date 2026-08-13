use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow constraints implied by other constraints.
    pub NO_REDUNDANT_TYPE_CONSTRAINT {
        id: "no-redundant-type-constraint",
        summary: "Disallow constraints implied by other constraints",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
