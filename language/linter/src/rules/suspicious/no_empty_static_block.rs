use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow empty static initialization blocks.
    pub NO_EMPTY_STATIC_BLOCK {
        id: "no-empty-static-block",
        code: "LU015",
        description: "Disallow empty static initialization blocks",
        category: Suspicious,
        level: Warning,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check no-empty-static-block.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
