use crate::rules::declare_lint;
use crate::{LinterError, MirModuleContext};

declare_lint! {
    /// Disallow loops that must exit during their first iteration.
    pub LOOP_SINGLE_ITERATION {
        id: "loop-single-iteration",
        code: "LU050",
        description: "Disallow loops that must exit during their first iteration",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check loop-single-iteration.
fn check(context: MirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
