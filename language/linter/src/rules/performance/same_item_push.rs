use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer bulk initialization over repeatedly appending the same value.
    pub SAME_ITEM_PUSH {
        id: "same-item-push",
        summary: "Prefer bulk initialization over repeatedly appending the same value",
        category: Performance,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check same-item-push.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
