use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow Promise.resolve calls that preserve the same Promise.
    pub NO_USELESS_PROMISE_RESOLVE {
        id: "no-useless-promise-resolve",
        summary: "Disallow Promise.resolve calls that preserve the same Promise",
        category: Suspicious,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check no-useless-promise-resolve.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
