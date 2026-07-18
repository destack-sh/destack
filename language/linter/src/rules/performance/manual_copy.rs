use crate::rules::declare_lint;
use crate::{LinterError, MirModuleContext};

declare_lint! {
    /// Replace element-by-element copy loops with a bulk copy operation.
    pub MANUAL_COPY {
        id: "manual-copy",
        code: "LP067",
        description: "Replace element-by-element copy loops with a bulk copy operation",
        category: Performance,
        level: Warning,
        fixable: Sometimes,
        check: MirModule(check),
    }
}

/// Check manual-copy.
fn check(context: MirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
