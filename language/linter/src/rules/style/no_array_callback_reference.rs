use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow passing function references directly to collection iteration methods.
    pub NO_ARRAY_CALLBACK_REFERENCE {
        id: "no-array-callback-reference",
        summary: "Disallow passing function references directly to collection iteration methods",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check no-array-callback-reference.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
