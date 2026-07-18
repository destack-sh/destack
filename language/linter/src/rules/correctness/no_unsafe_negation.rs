use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow negation of left operand in relational operators.
    pub NO_UNSAFE_NEGATION {
        id: "no-unsafe-negation",
        code: "LC034",
        description: "Disallow negation of left operand in relational operators",
        category: Correctness,
        level: Error,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check no-unsafe-negation.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
