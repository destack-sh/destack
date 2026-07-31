use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow overflow tests that perform the overflowing operation first.
    pub NO_OVERFLOW_CHECK_AFTER_OVERFLOW {
        id: "no-overflow-check-after-overflow",
        summary: "Disallow overflow tests that perform the overflowing operation first",
        category: Correctness,
        level: Error,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check no-overflow-check-after-overflow.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
