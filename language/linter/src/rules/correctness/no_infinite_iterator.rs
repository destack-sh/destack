use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow consuming provably unbounded iterators.
    pub NO_INFINITE_ITERATOR {
        id: "no-infinite-iterator",
        summary: "Disallow consuming provably unbounded iterators",
        category: Correctness,
        level: Error,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check no-infinite-iterator.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
