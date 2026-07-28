use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow values that only contribute to recursive calls of their own function.
    pub ONLY_USED_IN_RECURSION {
        id: "only-used-in-recursion",
        summary: "Disallow values that only contribute to recursive calls of their own function",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: MirModule(check),
    }
}

/// Check only-used-in-recursion.
fn check(_module: &MirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
