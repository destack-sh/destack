use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer copying iterator elements whose type is Copy.
    pub CLONED_INSTEAD_OF_COPIED {
        id: "cloned-instead-of-copied",
        summary: "Prefer copying iterator elements whose type is Copy",
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check cloned-instead-of-copied.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
