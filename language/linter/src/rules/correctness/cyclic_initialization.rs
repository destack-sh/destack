use crate::rules::declare_lint;
use crate::{LinterError, MirProgramContext};

declare_lint! {
    /// Disallow unsafe cycles in target initialization.
    pub CYCLIC_INITIALIZATION {
        id: "cyclic-initialization",
        code: "LC060",
        description: "Disallow unsafe cycles in target initialization",
        category: Correctness,
        level: Error,
        fixable: Never,
        check: MirProgram(check),
    }
}

/// Check cyclic-initialization.
fn check(context: MirProgramContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
