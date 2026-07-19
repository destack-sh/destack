use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow empty functions.
    pub NO_EMPTY_FUNCTION {
        id: "no-empty-function",
        description: "Disallow empty functions",
        category: Suspicious,
        level: Warning,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-empty-function.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
