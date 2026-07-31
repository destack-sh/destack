use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow borrows of owning containers that add needless indirection.
    pub NO_BORROWED_CONTAINER_PARAMETER {
        id: "no-borrowed-container-parameter",
        summary: "Disallow borrows of owning containers that add needless indirection",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check no-borrowed-container-parameter.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
