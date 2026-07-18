use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Prefer the exponentiation operator to a well-known power function.
    pub PREFER_EXPONENTIATION_OPERATOR {
        id: "prefer-exponentiation-operator",
        code: "LY036",
        description: "Prefer the exponentiation operator to a well-known power function",
        category: Style,
        level: Warning,
        fixable: Always,
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
