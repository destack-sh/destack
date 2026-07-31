use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow inconsistent implementations across equality, ordering, and hashing protocols.
    pub INCONSISTENT_EQUALITY_IMPLEMENTATION {
        id: "inconsistent-equality-implementation",
        summary: "Disallow inconsistent implementations across equality, ordering, and hashing protocols",
        category: Correctness,
        level: Error,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check inconsistent-equality-implementation.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
