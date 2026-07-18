use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Prefer tuple destructuring over indexed access.
    pub PREFER_TUPLE_DESTRUCTURE {
        id: "prefer-tuple-destructure",
        code: "LY059",
        description: "Prefer tuple destructuring over indexed access",
        category: Style,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check prefer-tuple-destructure.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
