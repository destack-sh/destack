use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow acquiring a lock while its guard is still live.
    pub NO_DOUBLE_LOCK {
        id: "no-double-lock",
        summary: "Disallow acquiring a lock while its guard is still live",
        category: Correctness,
        level: Error,
        fixable: None,
        check: MirModule(check),
    }
}

/// Check no-double-lock.
fn check(_module: &mut MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
