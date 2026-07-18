use crate::rules::declare_lint;
use crate::{LinterError, MirModuleContext};

declare_lint! {
    /// Warn when a large represented value is returned by value.
    pub LARGE_RETURN_BY_VALUE {
        id: "large-return-by-value",
        code: "LP054",
        description: "Warn when a large represented value is returned by value",
        category: Performance,
        level: Warning,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check large-return-by-value.
fn check(context: MirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
