use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow for loops with incorrect direction.
    pub FOR_DIRECTION {
        id: "for-direction",
        code: "LC001",
        description: "Disallow for loops with incorrect direction",
        category: Correctness,
        level: Error,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check for-direction.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
