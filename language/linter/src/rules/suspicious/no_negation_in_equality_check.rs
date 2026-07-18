use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintResult};

declare_lint! {
    /// Disallow negation in equality checks.
    pub NO_NEGATION_IN_EQUALITY_CHECK {
        id: "no-negation-in-equality-check",
        code: "LU024",
        description: "Disallow negation in equality checks",
        category: Suspicious,
        level: Warning,
        fixable: Sometimes,
        check: DirModule(check),
    }
}

/// Check no-negation-in-equality-check.
fn check(_module: &DirModule, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
