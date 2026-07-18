use crate::rules::declare_lint;
use crate::{DirModuleContext, LinterError};

declare_lint! {
    /// Require an explicit comparison function for generic collection sorting.
    pub REQUIRE_ARRAY_SORT_COMPARE {
        id: "require-array-sort-compare",
        code: "LC040",
        description: "Require an explicit comparison function for generic collection sorting",
        category: Correctness,
        level: Error,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check require-array-sort-compare.
fn check(context: DirModuleContext<'_>) -> Result<(), LinterError> {
    Err(LinterError::Unimplemented {
        lint: context.lint.definition.id.to_string(),
    })
}
