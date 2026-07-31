use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow adapters whose added capability is never used.
    pub UNUSED_PEEKABLE {
        id: "unused-peekable",
        summary: "Disallow adapters whose added capability is never used",
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: MirModule(check),
    }
}

/// Check unused-peekable.
fn check(_module: &MirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
