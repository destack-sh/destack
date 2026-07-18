use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow duplicate or covered else-if conditions.
    pub NO_DUPLICATE_ELSE_IF {
        id: "no-duplicate-else-if",
        code: "LU010",
        description: "Disallow duplicate or covered else-if conditions",
        category: Suspicious,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check no-duplicate-else-if.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
