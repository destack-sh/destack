use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Require async functions to suspend or return a Promise directly.
    pub REQUIRE_AWAIT {
        id: "require-await",
        code: "LU042",
        description: "Require async functions to suspend or return a Promise directly",
        category: Suspicious,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check require-await.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
