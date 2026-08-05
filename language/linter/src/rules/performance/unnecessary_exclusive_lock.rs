use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow exclusive locks on paths that only read.
    pub UNNECESSARY_EXCLUSIVE_LOCK {
        id: "unnecessary-exclusive-lock",
        summary: "Disallow exclusive locks on paths that only read",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: MirModule(check),
    }
}

/// Check unnecessary-exclusive-lock.
fn check(_module: &mut MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
