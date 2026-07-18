use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow assignment within an explicit or implicit return value.
    pub NO_RETURN_ASSIGN {
        id: "no-return-assign",
        code: "LU027",
        description: "Disallow assignment within an explicit or implicit return value",
        category: Suspicious,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check no-return-assign.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
