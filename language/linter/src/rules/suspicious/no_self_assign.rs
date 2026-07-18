use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow self-assignment.
    pub NO_SELF_ASSIGN {
        id: "no-self-assign",
        code: "LU028",
        description: "Disallow self-assignment",
        category: Suspicious,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check no-self-assign.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
