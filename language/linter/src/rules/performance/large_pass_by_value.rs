use crate::rules::declare_lint;
use crate::{LinterError, MirModuleContext};

declare_lint! {
    /// Warn when a large represented value is passed by value.
    pub LARGE_PASS_BY_VALUE {
        id: "large-pass-by-value",
        code: "LP052",
        description: "Warn when a large represented value is passed by value",
        category: Performance,
        level: Warning,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check large-pass-by-value.
fn check(context: MirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
