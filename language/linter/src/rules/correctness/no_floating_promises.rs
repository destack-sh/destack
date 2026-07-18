use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Require every Promise to be awaited, returned, or transferred to a scope.
    pub NO_FLOATING_PROMISES {
        id: "no-floating-promises",
        code: "LC016",
        description: "Require every Promise to be awaited, returned, or transferred to a scope",
        category: Correctness,
        level: Error,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check no-floating-promises.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
