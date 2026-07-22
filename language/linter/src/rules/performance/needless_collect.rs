use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow materializing a collection consumed by one streaming operation.
    pub NEEDLESS_COLLECT {
        id: "needless-collect",
        summary: "Disallow materializing a collection consumed by one streaming operation",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: MirModule(check),
    }
}

/// Check needless-collect.
fn check(_module: &MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
