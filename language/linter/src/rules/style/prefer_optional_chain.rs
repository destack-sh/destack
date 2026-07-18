use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Prefer optional chaining to equivalent guarded property access.
    pub PREFER_OPTIONAL_CHAIN {
        id: "prefer-optional-chain",
        code: "LY047",
        description: "Prefer optional chaining to equivalent guarded property access",
        category: Style,
        level: Warning,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check prefer-optional-chain.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
