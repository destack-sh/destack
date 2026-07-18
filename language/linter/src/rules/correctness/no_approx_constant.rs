use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow numeric literals that approximate well-known constants.
    pub NO_APPROX_CONSTANT {
        id: "no-approx-constant",
        code: "LC002",
        description: "Disallow numeric literals that approximate well-known constants",
        category: Correctness,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check no-approx-constant.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
