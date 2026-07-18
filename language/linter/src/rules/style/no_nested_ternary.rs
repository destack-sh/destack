use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow nested ternary expressions.
    pub NO_NESTED_TERNARY {
        id: "no-nested-ternary",
        code: "LY022",
        description: "Disallow nested ternary expressions",
        category: Style,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-nested-ternary.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
