use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintResult, MirModule};

declare_lint! {
    /// Warn when a value only contributes to recursive calls of its own function.
    pub ONLY_USED_IN_RECURSION {
        id: "only-used-in-recursion",
        description: "Warn when a value only contributes to recursive calls of its own function",
        category: Suspicious,
        level: Warning,
        fixable: Sometimes,
        check: MirModule(check),
    }
}

/// Check only-used-in-recursion.
fn check(_module: &MirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
