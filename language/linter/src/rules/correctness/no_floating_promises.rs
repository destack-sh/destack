use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Require every Promise to be awaited, returned, or transferred to a scope.
    pub NO_FLOATING_PROMISES {
        id: "no-floating-promises",
        summary: "Require every Promise to be awaited, returned, or transferred to a scope",
        category: Correctness,
        level: Error,
        fixable: Suggestion,
        check: MirModule(check),
    }
}

/// Check no-floating-promises.
fn check(_module: &MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
