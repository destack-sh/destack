use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow spin loops without suspension or a spin hint.
    pub BUSY_WAIT_WITHOUT_YIELD {
        id: "busy-wait-without-yield",
        summary: "Disallow spin loops without suspension or a spin hint",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: MirModule(check),
    }
}

/// Check busy-wait-without-yield.
fn check(_module: &MirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
