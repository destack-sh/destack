use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow comparisons with negative zero.
    pub NO_COMPARE_NEG_ZERO {
        id: "no-compare-neg-zero",
        summary: "Disallow comparisons with negative zero",
        category: Correctness,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check no-compare-neg-zero.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
