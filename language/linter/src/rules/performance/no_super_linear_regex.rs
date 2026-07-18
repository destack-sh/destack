use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow regex with potential catastrophic backtracking.
    pub NO_SUPER_LINEAR_REGEX {
        id: "no-super-linear-regex",
        code: "LP011",
        description: "Disallow regex with potential catastrophic backtracking",
        category: Performance,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-super-linear-regex.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
