use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow stored values that are never observed.
    pub DEAD_STORE {
        id: "dead-store",
        summary: "Disallow stored values that are never observed",
        category: Correctness,
        level: Warning,
        fixable: None,
        check: MirModule(check),
    }
}

/// Check dead-store.
fn check(_module: &MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
