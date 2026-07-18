use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow escape sequences that do not change the parsed value.
    pub NO_USELESS_ESCAPE {
        id: "no-useless-escape",
        code: "LU039",
        description: "Disallow escape sequences that do not change the parsed value",
        category: Suspicious,
        level: Warning,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check no-useless-escape.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
