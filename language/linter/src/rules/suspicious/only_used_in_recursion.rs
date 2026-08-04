use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow values that only contribute to recursive calls of their own function.
    pub ONLY_USED_IN_RECURSION {
        id: "only-used-in-recursion",
        summary: "Disallow values that only contribute to recursive calls of their own function",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check only-used-in-recursion.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
