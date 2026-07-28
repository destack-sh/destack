use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow aggregate Promise operations over one Promise.
    pub NO_SINGLE_PROMISE_IN_PROMISE_METHODS {
        id: "no-single-promise-in-promise-methods",
        summary: "Disallow aggregate Promise operations over one Promise",
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check no-single-promise-in-promise-methods.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
