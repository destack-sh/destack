use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Simplify boolean expressions with redundant or contradictory terms.
    pub NONMINIMAL_BOOL {
        id: "nonminimal-bool",
        summary: "Simplify boolean expressions with redundant or contradictory terms",
        category: Suspicious,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check nonminimal-bool.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
