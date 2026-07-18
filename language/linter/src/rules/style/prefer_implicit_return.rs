use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Prefer implicit return for arrow functions.
    pub PREFER_IMPLICIT_RETURN {
        id: "prefer-implicit-return",
        code: "LY041",
        description: "Prefer implicit return for arrow functions",
        category: Style,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check prefer-implicit-return.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
