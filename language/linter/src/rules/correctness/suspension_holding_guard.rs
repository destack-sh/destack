use crate::rules::declare_lint;
use crate::{LinterError, MirModuleContext};

declare_lint! {
    /// Disallow suspension while holding a guard or exclusive resource.
    pub SUSPENSION_HOLDING_GUARD {
        id: "suspension-holding-guard",
        code: "LC053",
        description: "Disallow suspension while holding a guard or exclusive resource",
        category: Correctness,
        level: Error,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check suspension-holding-guard.
fn check(context: MirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
