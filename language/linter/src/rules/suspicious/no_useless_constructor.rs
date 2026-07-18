use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow constructors that only repeat implicit construction behavior.
    pub NO_USELESS_CONSTRUCTOR {
        id: "no-useless-constructor",
        code: "LU038",
        description: "Disallow constructors that only repeat implicit construction behavior",
        category: Suspicious,
        level: Warning,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check no-useless-constructor.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
