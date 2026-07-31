use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow allocating conversions performed only to compare.
    pub NO_ALLOCATION_FOR_COMPARISON {
        id: "no-allocation-for-comparison",
        summary: "Disallow allocating conversions performed only to compare",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check no-allocation-for-comparison.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
