use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow control characters in regex.
    pub NO_CONTROL_REGEX {
        id: "no-control-regex",
        code: "LC010",
        description: "Disallow control characters in regex",
        category: Correctness,
        level: Error,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-control-regex.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
