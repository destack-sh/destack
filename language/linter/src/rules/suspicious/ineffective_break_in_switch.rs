use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow breaks that leave a switch where the loop was intended.
    pub INEFFECTIVE_BREAK_IN_SWITCH {
        id: "ineffective-break-in-switch",
        summary: "Disallow breaks that leave a switch where the loop was intended",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
