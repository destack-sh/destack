use crate::rules::declare_lint;
use crate::{DirProgramContext, LinterError};

declare_lint! {
    /// Warn on declared dependencies unused by the target program.
    pub UNUSED_DEPENDENCY {
        id: "unused-dependency",
        code: "LU063",
        description: "Warn on declared dependencies unused by the target program",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirProgram(check),
    }
}

/// Check unused-dependency.
fn check(context: DirProgramContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
