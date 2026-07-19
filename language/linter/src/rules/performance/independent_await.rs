use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirModule};

declare_lint! {
    /// Warn when independent asynchronous operations are awaited sequentially.
    pub INDEPENDENT_AWAIT {
        id: "independent-await",
        description: "Warn when independent asynchronous operations are awaited sequentially",
        category: Performance,
        level: Warning,
        fixable: Sometimes,
        check: MirModule(check),
    }
}

/// Check independent-await.
fn check(_module: &MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
