use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow focused tests.
    pub NO_FOCUSED_TEST {
        id: "no-focused-test",
        summary: "Disallow focused tests",
        category: Suspicious,
        level: Error,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check no-focused-test.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
