use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Require explicit types at exported and public API boundaries.
    pub EXPLICIT_PUBLIC_TYPES {
        id: "explicit-public-types",
        code: "LY011",
        description: "Require explicit types at exported and public API boundaries",
        category: Style,
        level: Warning,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check explicit-public-types.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
