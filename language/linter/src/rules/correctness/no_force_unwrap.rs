use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow force unwrapping values that may be absent or failed.
    pub NO_FORCE_UNWRAP {
        id: "no-force-unwrap",
        summary: "Disallow force unwrapping values that may be absent or failed",
        category: Correctness,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check no-force-unwrap.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
