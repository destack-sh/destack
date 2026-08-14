use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow type parameters whose constraints can replace them without changing behavior.
    pub NO_UNNECESSARY_TYPE_PARAMETER {
        id: "no-unnecessary-type-parameter",
        summary: "Disallow type parameters that add no relationship or precision",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule,
    }
}
