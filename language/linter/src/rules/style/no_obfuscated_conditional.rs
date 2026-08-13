use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Disallow conditional chains obscuring a simple branch.
    pub NO_OBFUSCATED_CONDITIONAL {
        id: "no-obfuscated-conditional",
        summary: "Disallow conditional chains obscuring a simple branch",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}
