use crate::rules::declare_lint;
use crate::{LinterError, MirModuleContext};

declare_lint! {
    /// Disallow clones proven unnecessary by ownership and liveness.
    pub REDUNDANT_CLONE {
        id: "redundant-clone",
        code: "LP050",
        description: "Disallow clones proven unnecessary by ownership and liveness",
        category: Performance,
        level: Warning,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check redundant-clone.
fn check(context: MirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
