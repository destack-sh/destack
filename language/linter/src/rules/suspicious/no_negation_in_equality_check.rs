use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow negation in equality checks.
    pub NO_NEGATION_IN_EQUALITY_CHECK {
        id: "no-negation-in-equality-check",
        code: "LU024",
        description: "Disallow negation in equality checks",
        category: Suspicious,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check no-negation-in-equality-check.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
