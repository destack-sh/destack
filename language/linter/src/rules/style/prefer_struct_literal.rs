use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Prefer struct literal form.
    pub PREFER_STRUCT_LITERAL {
        id: "prefer-struct-literal",
        code: "LY056",
        description: "Prefer struct literal form",
        category: Style,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check prefer-struct-literal.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
