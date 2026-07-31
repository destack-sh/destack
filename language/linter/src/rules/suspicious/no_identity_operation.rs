use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow arithmetic with an operand that cannot change the result.
    pub NO_IDENTITY_OPERATION {
        id: "no-identity-operation",
        summary: "Disallow arithmetic with an operand that cannot change the result",
        category: Suspicious,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check no-identity-operation.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
