use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Prefer tuple assignment for swaps.
    pub PREFER_TUPLE_SWAP {
        id: "prefer-tuple-swap",
        description: "Prefer tuple assignment for swaps",
        category: Style,
        level: Warning,
        fixable: Always,
        check: DirModule(check),
    }
}

/// Check prefer-tuple-swap.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
