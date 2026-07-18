use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow switch cases that reproduce default behavior.
    pub NO_USELESS_SWITCH_CASE {
        id: "no-useless-switch-case",
        code: "LU048",
        description: "Disallow switch cases that reproduce default behavior",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-useless-switch-case.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
