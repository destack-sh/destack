use crate::rules::declare_lint;
use crate::{LinterError, MirModuleContext};

declare_lint! {
    /// Disallow loops whose condition dependencies never change.
    pub UNMODIFIED_LOOP_CONDITION {
        id: "unmodified-loop-condition",
        code: "LC051",
        description: "Disallow loops whose condition dependencies never change",
        category: Correctness,
        level: Error,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check unmodified-loop-condition.
fn check(context: MirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
