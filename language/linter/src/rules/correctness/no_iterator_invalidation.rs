use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirModule};

declare_lint! {
    /// Disallow invalidating a collection while one of its iterators remains live.
    pub NO_ITERATOR_INVALIDATION {
        id: "no-iterator-invalidation",
        description: "Disallow invalidating a collection while one of its iterators remains live",
        category: Correctness,
        level: Error,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check no-iterator-invalidation.
fn check(_module: &MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
