use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer join over collecting formatted pieces.
    pub PREFER_JOIN_OVER_STRING_COLLECT {
        id: "prefer-join-over-string-collect",
        summary: "Prefer join over collecting formatted pieces",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check prefer-join-over-string-collect.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
