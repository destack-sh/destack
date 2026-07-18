use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Prefer unary negation over multiplying by -1.
    pub PREFER_UNARY_NEGATION {
        id: "prefer-unary-negation",
        code: "LY061",
        description: "Prefer unary negation over multiplying by -1",
        category: Style,
        level: Warning,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check prefer-unary-negation.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
