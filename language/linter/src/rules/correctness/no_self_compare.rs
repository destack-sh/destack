use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow comparisons of a value with itself.
    pub NO_SELF_COMPARE {
        id: "no-self-compare",
        description: "Disallow comparisons of a value with itself",
        category: Correctness,
        level: Error,
        fixable: Never,
        check: DirModule(check),
    }
}

/// Check no-self-compare.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
