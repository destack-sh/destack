use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow length checks duplicated by the guarded operation.
    pub NO_USELESS_LENGTH_CHECK {
        id: "no-useless-length-check",
        code: "LU047",
        description: "Disallow length checks duplicated by the guarded operation",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-useless-length-check.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
