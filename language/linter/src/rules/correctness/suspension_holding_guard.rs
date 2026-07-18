use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirModule};

declare_lint! {
    /// Disallow suspension while holding a guard or exclusive resource.
    pub SUSPENSION_HOLDING_GUARD {
        id: "suspension-holding-guard",
        code: "LC053",
        description: "Disallow suspension while holding a guard or exclusive resource",
        category: Correctness,
        level: Error,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check suspension-holding-guard.
fn check(_module: &MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
