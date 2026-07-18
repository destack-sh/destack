use crate::rules::declare_lint;
use crate::{DirProgramContext, LinterError};

declare_lint! {
    /// Disallow substantial alpha-equivalent checked code.
    pub NO_DUPLICATE_CODE {
        id: "no-duplicate-code",
        code: "LU060",
        description: "Disallow substantial alpha-equivalent checked code",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirProgram(check),
    }
}

/// Check no-duplicate-code.
fn check(context: DirProgramContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
