use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow repeated spreading into an accumulating value.
    pub NO_ACCUMULATING_SPREAD {
        id: "no-accumulating-spread",
        summary: "Disallow repeated spreading into an accumulating value",
        category: Performance,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check no-accumulating-spread.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
