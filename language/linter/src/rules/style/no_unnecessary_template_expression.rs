use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow template expressions without formatting or conversion.
    pub NO_UNNECESSARY_TEMPLATE_EXPRESSION {
        id: "no-unnecessary-template-expression",
        code: "LY068",
        description: "Disallow template expressions without formatting or conversion",
        category: Style,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-unnecessary-template-expression.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
