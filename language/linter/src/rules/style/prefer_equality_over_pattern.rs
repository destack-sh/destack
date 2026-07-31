use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer equality over matching a complete literal pattern.
    pub PREFER_EQUALITY_OVER_PATTERN {
        id: "prefer-equality-over-pattern",
        summary: "Prefer equality over matching a complete literal pattern",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check prefer-equality-over-pattern.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
