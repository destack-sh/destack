use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer for-of over repeatedly requesting the next iterator value.
    pub WHILE_LET_ON_ITERATOR {
        id: "while-let-on-iterator",
        summary: "Prefer for-of over repeatedly requesting the next iterator value",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check while-let-on-iterator.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
