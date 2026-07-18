use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow credentials and secret material embedded in source values.
    pub HARDCODED_SECRET {
        id: "hardcoded-secret",
        code: "LS009",
        description: "Disallow credentials and secret material embedded in source values",
        category: Security,
        level: Error,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check hardcoded-secret.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
