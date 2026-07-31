use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow comparators that reproduce the natural ordering.
    pub UNNECESSARY_SORT_COMPARATOR {
        id: "unnecessary-sort-comparator",
        summary: "Disallow comparators that reproduce the natural ordering",
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check unnecessary-sort-comparator.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
