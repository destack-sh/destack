use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirModule};

declare_lint! {
    /// Warn when values retained across suspension create a large coroutine.
    pub LARGE_COROUTINE_STATE {
        id: "large-coroutine-state",
        description: "Warn when values retained across suspension create a large coroutine",
        category: Performance,
        level: Warning,
        fixable: Never,
        check: MirModule(check),
    }
}

/// Check large-coroutine-state.
fn check(_module: &MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
