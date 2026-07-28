use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow excessive coroutine state retained across suspension.
    pub LARGE_COROUTINE_STATE {
        id: "large-coroutine-state",
        summary: "Disallow excessive coroutine state retained across suspension",
        category: Performance,
        level: Warning,
        fixable: None,
        check: MirModule(check),
    }
}

/// Check large-coroutine-state.
fn check(_module: &MirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
