use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow guards expressible inside their pattern.
    pub NO_REDUNDANT_MATCH_GUARD {
        id: "no-redundant-match-guard",
        summary: "Disallow guards expressible inside their pattern",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
