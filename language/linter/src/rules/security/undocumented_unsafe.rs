use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Require a safety rationale for every unsafe declaration and expression.
    pub UNDOCUMENTED_UNSAFE {
        id: "undocumented-unsafe",
        code: "LS011",
        description: "Require a safety rationale for every unsafe declaration and expression",
        category: Security,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check undocumented-unsafe.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
