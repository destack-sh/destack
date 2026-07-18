use crate::rules::declare_lint;
use crate::{LinterError, MirModuleContext};

declare_lint! {
    /// Warn when a parameter is never consumed or mutated.
    pub NEEDLESS_PASS_BY_VALUE {
        id: "needless-pass-by-value",
        code: "LP053",
        description: "Warn when a parameter is never consumed or mutated",
        category: Performance,
        level: Warning,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check needless-pass-by-value.
fn check(context: MirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
