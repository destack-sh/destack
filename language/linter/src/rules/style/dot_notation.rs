use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Prefer dot notation for property access.
    pub DOT_NOTATION {
        id: "dot-notation",
        code: "LY009",
        description: "Prefer dot notation for property access",
        category: Style,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check dot-notation.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
