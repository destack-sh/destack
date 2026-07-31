use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow explicit iterator calls in for-of.
    pub NO_EXPLICIT_ITERATOR_IN_FOR_OF {
        id: "no-explicit-iterator-in-for-of",
        summary: "Disallow explicit iterator calls in for-of",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check no-explicit-iterator-in-for-of.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
