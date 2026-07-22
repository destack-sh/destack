use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer eager fallbacks when deferred evaluation cannot avoid work.
    pub UNNECESSARY_LAZY_EVALUATIONS {
        id: "unnecessary-lazy-evaluations",
        summary: "Prefer eager fallbacks when deferred evaluation cannot avoid work",
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check unnecessary-lazy-evaluations.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
