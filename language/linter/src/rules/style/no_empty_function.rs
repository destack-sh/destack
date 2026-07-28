use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow empty functions.
    pub NO_EMPTY_FUNCTION {
        id: "no-empty-function",
        summary: "Disallow empty functions",
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check no-empty-function.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
