use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow duplicate or covered else-if conditions.
    pub NO_DUPLICATE_ELSE_IF {
        id: "no-duplicate-else-if",
        description: "Disallow duplicate or covered else-if conditions",
        category: Suspicious,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check no-duplicate-else-if.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
