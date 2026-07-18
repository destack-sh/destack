use crate::rules::declare_lint;
use crate::{LinterError, MirModuleContext};

declare_lint! {
    /// Disallow materializing a collection consumed by one streaming operation.
    pub NEEDLESS_COLLECT {
        id: "needless-collect",
        code: "LP064",
        description: "Disallow materializing a collection consumed by one streaming operation",
        category: Performance,
        level: Warning,
        fixable: Sometimes,
        check: MirModule(check),
    }
}

/// Check needless-collect.
fn check(context: MirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
