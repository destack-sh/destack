use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow suspension while holding a guard or exclusive resource.
    pub SUSPENSION_HOLDING_GUARD {
        id: "suspension-holding-guard",
        summary: "Disallow suspension while holding a guard or exclusive resource",
        category: Correctness,
        level: Error,
        fixable: None,
        check: MirModule(check),
    }
}

/// Check suspension-holding-guard.
fn check(_module: &mut MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
