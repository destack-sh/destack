use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Warn when argument names indicate swapped positional arguments.
    pub SWAPPED_ARGUMENTS {
        id: "swapped-arguments",
        code: "LC046",
        description: "Warn when argument names indicate swapped positional arguments",
        category: Correctness,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check swapped-arguments.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
