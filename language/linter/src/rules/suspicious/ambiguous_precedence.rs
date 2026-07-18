use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Require grouping where operator precedence is easy to misread.
    pub AMBIGUOUS_PRECEDENCE {
        id: "ambiguous-precedence",
        code: "LU001",
        description: "Require grouping where operator precedence is easy to misread",
        category: Suspicious,
        level: Warning,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check ambiguous-precedence.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
