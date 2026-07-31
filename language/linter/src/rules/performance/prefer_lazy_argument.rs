use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer lazy fallbacks when eager arguments perform avoidable work.
    pub PREFER_LAZY_ARGUMENT {
        id: "prefer-lazy-argument",
        summary: "Prefer lazy fallbacks when eager arguments perform avoidable work",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check prefer-lazy-argument.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
