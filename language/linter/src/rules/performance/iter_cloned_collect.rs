use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer direct collection cloning over iterating, cloning, and collecting.
    pub ITER_CLONED_COLLECT {
        id: "iter-cloned-collect",
        summary: "Prefer direct collection cloning over iterating, cloning, and collecting",
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check iter-cloned-collect.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
