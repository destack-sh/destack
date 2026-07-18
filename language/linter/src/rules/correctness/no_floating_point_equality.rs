use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow direct == comparison of floats.
    pub NO_FLOATING_POINT_EQUALITY {
        id: "no-floating-point-equality",
        code: "LC015",
        description: "Disallow direct == comparison of floats",
        category: Correctness,
        level: Error,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-floating-point-equality.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
