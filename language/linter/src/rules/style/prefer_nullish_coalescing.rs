use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Prefer nullish coalescing when only nullish values select the fallback.
    pub PREFER_NULLISH_COALESCING {
        id: "prefer-nullish-coalescing",
        code: "LY046",
        description: "Prefer nullish coalescing when only nullish values select the fallback",
        category: Style,
        level: Warning,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check prefer-nullish-coalescing.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
