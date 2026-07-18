use crate::rules::declare_lint;
use crate::{LinterError, MirModuleContext};

declare_lint! {
    /// Disallow functions that recurse before every possible return.
    pub UNCONDITIONAL_RECURSION {
        id: "unconditional-recursion",
        code: "LC050",
        description: "Disallow functions that recurse before every possible return",
        category: Correctness,
        level: Error,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check unconditional-recursion.
fn check(context: MirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
