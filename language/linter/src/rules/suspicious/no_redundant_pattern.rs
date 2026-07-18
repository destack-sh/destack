use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow patterns that bind nothing.
    pub NO_REDUNDANT_PATTERN {
        id: "no-redundant-pattern",
        code: "LU026",
        description: "Disallow patterns that bind nothing",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-redundant-pattern.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
