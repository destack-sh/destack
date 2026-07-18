use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Disallow comparisons of a value with itself.
    pub NO_SELF_COMPARE {
        id: "no-self-compare",
        code: "LC025",
        description: "Disallow comparisons of a value with itself",
        category: Correctness,
        level: Error,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-self-compare.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
