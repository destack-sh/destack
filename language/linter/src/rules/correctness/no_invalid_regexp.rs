use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow constant regex sources that fail to compile.
    pub NO_INVALID_REGEXP {
        id: "no-invalid-regexp",
        summary: "Disallow constant regex sources that fail to compile",
        category: Correctness,
        level: Error,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check no-invalid-regexp.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
