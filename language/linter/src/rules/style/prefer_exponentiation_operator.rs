use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Prefer the exponentiation operator to a well-known power function.
    pub PREFER_EXPONENTIATION_OPERATOR {
        id: "prefer-exponentiation-operator",
        code: "LY036",
        description: "Prefer the exponentiation operator to a well-known power function",
        category: Style,
        level: Warning,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check prefer-exponentiation-operator.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
