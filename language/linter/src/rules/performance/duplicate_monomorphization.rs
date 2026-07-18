use crate::rules::declare_lint;
use crate::{LinterError, MirProgramContext};

declare_lint! {
    /// Warn when equivalent generic instances duplicate generated program work.
    pub DUPLICATE_MONOMORPHIZATION {
        id: "duplicate-monomorphization",
        code: "LP070",
        description: "Warn when equivalent generic instances duplicate generated program work",
        category: Performance,
        level: Warning,
        fixable: Never,
        check: MirProgram(check),
    }
}

/// Check duplicate-monomorphization.
fn check(context: MirProgramContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
