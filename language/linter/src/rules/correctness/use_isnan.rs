use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Require the NaN predicate instead of equality comparisons with NaN.
    pub USE_ISNAN {
        id: "use-isnan",
        code: "LC043",
        description: "Require the NaN predicate instead of equality comparisons with NaN",
        category: Correctness,
        level: Error,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check use-isnan.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
