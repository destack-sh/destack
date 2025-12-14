use crate::declare_lint;
use crate::linter::LintContext;

declare_lint! {
    /// Disallow debugger statements in production code.
    ///
    /// Debugger statements should not be committed to production code
    /// as they can cause the program to pause unexpectedly.
    #[lint(
        id = "no-debugger",
        code = "LC001",
        category = Correctness,
        level = Dir,
        fixable,
        docs = "https://destack.dev/lint/no-debugger",
    )]
    pub NoDebugger,
    "Disallow debugger statements"
}

impl NoDebugger {
    /// Check a module for debugger statements.
    pub fn check(&self, ctx: &mut LintContext<'_>) {
        // TODO #Incomplete: implement actual check for debugger statements
        // for now this is a placeholder to demonstrate the lint system
        let _ = ctx;
    }
}
