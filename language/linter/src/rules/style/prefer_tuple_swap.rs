use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer tuple assignment for swaps.
    pub PREFER_TUPLE_SWAP {
        id: "prefer-tuple-swap",
        summary: "Prefer tuple assignment for swaps",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check prefer-tuple-swap.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
