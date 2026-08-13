use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow spreads that have no observable effect.
    pub NO_USELESS_SPREAD {
        id: "no-useless-spread",
        summary: "Disallow spreads that have no observable effect",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
