use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow full-range slicing where a borrow suffices.
    pub NO_REDUNDANT_FULL_SLICE {
        id: "no-redundant-full-slice",
        summary: "Disallow full-range slicing where a borrow suffices",
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check no-redundant-full-slice.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
