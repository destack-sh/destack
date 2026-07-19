use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Require every Promise to be awaited, returned, or transferred to a scope.
    pub NO_FLOATING_PROMISES {
        id: "no-floating-promises",
        description: "Require every Promise to be awaited, returned, or transferred to a scope",
        category: Correctness,
        level: Error,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check no-floating-promises.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
