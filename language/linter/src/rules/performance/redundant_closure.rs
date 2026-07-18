use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow closures that only forward their arguments to another callable.
    pub REDUNDANT_CLOSURE {
        id: "redundant-closure",
        code: "LP065",
        description: "Disallow closures that only forward their arguments to another callable",
        category: Performance,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check redundant-closure.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
