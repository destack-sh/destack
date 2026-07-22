use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer RegExp.exec when regular-expression match details are consumed.
    pub PREFER_REGEXP_EXEC {
        id: "prefer-regexp-exec",
        summary: "Prefer RegExp.exec when regular-expression match details are consumed",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check prefer-regexp-exec.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
