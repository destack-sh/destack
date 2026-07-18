use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Require compound assignment where equivalent.
    pub OPERATOR_ASSIGNMENT {
        id: "operator-assignment",
        code: "LY028",
        description: "Require compound assignment where equivalent",
        category: Style,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check operator-assignment.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
