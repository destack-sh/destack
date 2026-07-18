use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow comparing to boolean literals.
    pub NO_BOOLEAN_LITERAL_COMPARE {
        id: "no-boolean-literal-compare",
        code: "LY014",
        description: "Disallow comparing to boolean literals",
        category: Style,
        level: Warning,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check no-boolean-literal-compare.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
