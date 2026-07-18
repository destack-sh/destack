use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow void expressions that accomplish nothing.
    pub NO_MEANINGLESS_VOID {
        id: "no-meaningless-void",
        code: "LU049",
        description: "Disallow void expressions that accomplish nothing",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-meaningless-void.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
