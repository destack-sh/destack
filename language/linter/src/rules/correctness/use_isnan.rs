use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Require the NaN predicate instead of equality comparisons with NaN.
    pub USE_ISNAN {
        id: "use-isnan",
        description: "Require the NaN predicate instead of equality comparisons with NaN",
        category: Correctness,
        level: Error,
        fixable: Sometimes,
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
