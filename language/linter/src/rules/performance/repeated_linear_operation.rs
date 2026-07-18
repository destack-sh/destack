use crate::rules::declare_lint;
use crate::{LinterError, MirModuleContext};

declare_lint! {
    /// Warn when a linear operation is nested in repeated execution.
    pub REPEATED_LINEAR_OPERATION {
        id: "repeated-linear-operation",
        code: "LP058",
        description: "Warn when a linear operation is nested in repeated execution",
        category: Performance,
        level: Warning,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check repeated-linear-operation.
fn check(context: MirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
