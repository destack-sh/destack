use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Suggest merging nested if statements.
    pub NO_COLLAPSIBLE_IF {
        id: "no-collapsible-if",
        code: "LY015",
        description: "Suggest merging nested if statements",
        category: Style,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check no-collapsible-if.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
