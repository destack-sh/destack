use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow unnecessary ternary expressions.
    pub NO_UNNEEDED_TERNARY {
        id: "no-unneeded-ternary",
        code: "LY025",
        description: "Disallow unnecessary ternary expressions",
        category: Style,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check no-unneeded-ternary.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
