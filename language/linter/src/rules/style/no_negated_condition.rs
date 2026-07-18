use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Prefer positive conditions when both branches are present.
    pub NO_NEGATED_CONDITION {
        id: "no-negated-condition",
        code: "LY020",
        description: "Prefer positive conditions when both branches are present",
        category: Style,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check no-negated-condition.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
