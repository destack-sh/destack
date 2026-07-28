use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow cloning iterator elements that are only borrowed afterward.
    pub REDUNDANT_ITER_CLONED {
        id: "redundant-iter-cloned",
        summary: "Disallow cloning iterator elements that are only borrowed afterward",
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check redundant-iter-cloned.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
