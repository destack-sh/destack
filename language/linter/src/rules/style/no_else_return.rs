use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow else branches after an unconditional control transfer.
    pub NO_ELSE_RETURN {
        id: "no-else-return",
        code: "LY017",
        description: "Disallow else branches after an unconditional control transfer",
        category: Style,
        level: Warning,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check no-else-return.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
