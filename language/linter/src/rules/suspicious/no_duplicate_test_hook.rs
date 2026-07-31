use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow repeated lifecycle hooks in one suite.
    pub NO_DUPLICATE_TEST_HOOK {
        id: "no-duplicate-test-hook",
        summary: "Disallow repeated lifecycle hooks in one suite",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check no-duplicate-test-hook.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
