use crate::rules::declare_lint;
use crate::{LinterError, MirModuleContext};

declare_lint! {
    /// Warn when a value only contributes to recursive calls of its own function.
    pub ONLY_USED_IN_RECURSION {
        id: "only-used-in-recursion",
        code: "LU033",
        description: "Warn when a value only contributes to recursive calls of its own function",
        category: Suspicious,
        level: Warning,
        fixable: Sometimes,
        check: MirModule(check),
    }
}

/// Check only-used-in-recursion.
fn check(context: MirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
