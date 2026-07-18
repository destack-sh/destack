use crate::rules::declare_lint;
use crate::{LinterError, MirModuleContext};

declare_lint! {
    /// Disallow borrows that add no required lifetime or capability.
    pub NEEDLESS_BORROW {
        id: "needless-borrow",
        code: "LP051",
        description: "Disallow borrows that add no required lifetime or capability",
        category: Performance,
        level: Warning,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check needless-borrow.
fn check(context: MirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
