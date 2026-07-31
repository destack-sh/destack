use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow key types whose hash or equality depends on mutable state.
    pub NO_MUTABLE_MAP_KEY {
        id: "no-mutable-map-key",
        summary: "Disallow key types whose hash or equality depends on mutable state",
        category: Correctness,
        level: Error,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check no-mutable-map-key.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
