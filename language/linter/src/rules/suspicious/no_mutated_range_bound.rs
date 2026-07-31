use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{Lint, LintResult, MirModule};

declare_lint_stub! {
    /// Disallow mutating range bounds that cannot affect iteration.
    pub NO_MUTATED_RANGE_BOUND {
        id: "no-mutated-range-bound",
        summary: "Disallow mutating range bounds that cannot affect iteration",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: MirModule(check),
    }
}

/// Check no-mutated-range-bound.
fn check(_module: &MirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
