use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Require an explicit comparison function for generic collection sorting.
    pub REQUIRE_ARRAY_SORT_COMPARE {
        id: "require-array-sort-compare",
        description: "Require an explicit comparison function for generic collection sorting",
        category: Correctness,
        level: Error,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check require-array-sort-compare.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
