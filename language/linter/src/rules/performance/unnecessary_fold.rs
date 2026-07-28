use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer a specialized iterator operation over an equivalent fold.
    pub UNNECESSARY_FOLD {
        id: "unnecessary-fold",
        summary: "Prefer a specialized iterator operation over an equivalent fold",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check unnecessary-fold.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
