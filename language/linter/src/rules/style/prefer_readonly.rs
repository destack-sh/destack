use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Require readonly for fields never mutated after initialization.
    pub PREFER_READONLY {
        id: "prefer-readonly",
        code: "LY078",
        description: "Require readonly for fields never mutated after initialization",
        category: Style,
        level: Warning,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check prefer-readonly.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
