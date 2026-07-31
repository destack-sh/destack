use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow mutation inside debug-only assertions.
    pub NO_SIDE_EFFECT_IN_DEBUG_ASSERT {
        id: "no-side-effect-in-debug-assert",
        summary: "Disallow mutation inside debug-only assertions",
        category: Correctness,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check no-side-effect-in-debug-assert.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
