use crate::rules::declare_lint;
use crate::{LinterError, MirModuleContext};

declare_lint! {
    /// Disallow cloning values whose checked type is Copy.
    pub CLONE_ON_COPY {
        id: "clone-on-copy",
        code: "LP062",
        description: "Disallow cloning values whose checked type is Copy",
        category: Performance,
        level: Warning,
        fixable: Always,
        check: MirModule(check),
    }
}

/// Check clone-on-copy.
fn check(context: MirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
