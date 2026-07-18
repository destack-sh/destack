use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Require the canonical extension declaration form for its visibility.
    pub CONSISTENT_EXTENSION_STYLE {
        id: "consistent-extension-style",
        code: "LY005",
        description: "Require the canonical extension declaration form for its visibility",
        category: Style,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check consistent-extension-style.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
