use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow void expressions that accomplish nothing.
    pub NO_MEANINGLESS_VOID {
        id: "no-meaningless-void",
        summary: "Disallow void expressions that accomplish nothing",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule,
    }
}
