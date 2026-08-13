use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Prefer map over chaining that rewraps every present case.
    pub PREFER_MAP_OVER_AND_THEN {
        id: "prefer-map-over-and-then",
        summary: "Prefer map over chaining that rewraps every present case",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
