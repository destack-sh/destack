use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow awaiting inside concurrent combinator arguments.
    pub NO_AWAIT_IN_CONCURRENT_COMBINATOR {
        id: "no-await-in-concurrent-combinator",
        summary: "Disallow awaiting inside concurrent combinator arguments",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check no-await-in-concurrent-combinator.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
