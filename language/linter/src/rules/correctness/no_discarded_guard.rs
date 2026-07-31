use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow binding guards to discard patterns that release them immediately.
    pub NO_DISCARDED_GUARD {
        id: "no-discarded-guard",
        summary: "Disallow binding guards to discard patterns that release them immediately",
        category: Correctness,
        level: Error,
        fixable: None,
        check: MirModule(check),
    }
}

/// Check no-discarded-guard.
fn check(_module: &MirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
