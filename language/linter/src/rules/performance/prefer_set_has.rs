use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Prefer set membership for repeated linear membership tests.
    pub PREFER_SET_HAS {
        id: "prefer-set-has",
        summary: "Prefer set membership for repeated linear membership tests",
        category: Performance,
        level: Warning,
        fixable: None,
        check: MirModule(check),
    }
}

/// Check prefer-set-has.
fn check(_module: &mut MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
