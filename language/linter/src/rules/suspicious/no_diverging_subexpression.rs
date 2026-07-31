use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow diverging subexpressions inside larger expressions.
    pub NO_DIVERGING_SUBEXPRESSION {
        id: "no-diverging-subexpression",
        summary: "Disallow diverging subexpressions inside larger expressions",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check no-diverging-subexpression.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
