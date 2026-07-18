use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow identical checked conditional branches.
    pub NO_IDENTICAL_BRANCHES {
        id: "no-identical-branches",
        code: "LU018",
        description: "Disallow identical checked conditional branches",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-identical-branches.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
