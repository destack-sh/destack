use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer the exponentiation operator to a well-known power function.
    pub PREFER_EXPONENTIATION_OPERATOR {
        id: "prefer-exponentiation-operator",
        summary: "Prefer the exponentiation operator to a well-known power function",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check prefer-exponentiation-operator.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
