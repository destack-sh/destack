use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Delay iterator cloning until after operations that discard elements.
    pub ITER_OVEREAGER_CLONED {
        id: "iter-overeager-cloned",
        summary: "Delay iterator cloning until after operations that discard elements",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check iter-overeager-cloned.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
