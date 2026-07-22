use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Require the NaN predicate instead of equality comparisons with NaN.
    pub USE_ISNAN {
        id: "use-isnan",
        summary: "Require the NaN predicate instead of equality comparisons with NaN",
        category: Correctness,
        level: Error,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check use-isnan.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
