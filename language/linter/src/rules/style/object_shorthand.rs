use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Require object property shorthand where equivalent.
    pub OBJECT_SHORTHAND {
        id: "object-shorthand",
        code: "LY027",
        description: "Require object property shorthand where equivalent",
        category: Style,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check object-shorthand.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
