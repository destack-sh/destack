use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow invisible and misleading characters in source text.
    pub NO_INVISIBLE_CHARACTER {
        id: "no-invisible-character",
        summary: "Disallow invisible and misleading characters in source text",
        category: Security,
        level: Error,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check no-invisible-character.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
