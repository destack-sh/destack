use crate::rules::declare_lint;
use crate::{LinterError, MirModuleContext};

declare_lint! {
    /// Warn when independent asynchronous operations are awaited sequentially.
    pub INDEPENDENT_AWAIT {
        id: "independent-await",
        code: "LP066",
        description: "Warn when independent asynchronous operations are awaited sequentially",
        category: Performance,
        level: Warning,
        fixable: Sometimes,
        check: MirModule(check),
    }
}

/// Check independent-await.
fn check(context: MirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
