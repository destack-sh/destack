use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow negated float comparisons that mishandle NaN.
    pub NO_NEGATED_FLOAT_COMPARISON {
        id: "no-negated-float-comparison",
        summary: "Disallow negated float comparisons that mishandle NaN",
        category: Correctness,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check no-negated-float-comparison.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
