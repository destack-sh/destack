use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow switch cases that reproduce default behavior.
    pub NO_USELESS_SWITCH_CASE {
        id: "no-useless-switch-case",
        summary: "Disallow switch cases that reproduce default behavior",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
