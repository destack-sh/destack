use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Prefer expression initialization over staged assignment.
    pub PREFER_EXPRESSION {
        id: "prefer-expression",
        code: "LY037",
        description: "Prefer expression initialization over staged assignment",
        category: Style,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check prefer-expression.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
