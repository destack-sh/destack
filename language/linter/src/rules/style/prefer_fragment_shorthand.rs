use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Prefer fragment shorthand when no fragment properties are present.
    pub PREFER_FRAGMENT_SHORTHAND {
        id: "prefer-fragment-shorthand",
        code: "LY039",
        description: "Prefer fragment shorthand when no fragment properties are present",
        category: Style,
        level: Warning,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check prefer-fragment-shorthand.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
