use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow awaiting values outside the canonical Promise domain.
    pub AWAIT_THENABLE {
        id: "await-thenable",
        code: "LC056",
        description: "Disallow awaiting values outside the canonical Promise domain",
        category: Correctness,
        level: Error,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check await-thenable.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
