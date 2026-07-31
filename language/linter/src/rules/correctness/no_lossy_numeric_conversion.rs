use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow narrowing conversions whose input range is not proven to fit.
    pub NO_LOSSY_NUMERIC_CONVERSION {
        id: "no-lossy-numeric-conversion",
        summary: "Disallow narrowing conversions whose input range is not proven to fit",
        category: Correctness,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check no-lossy-numeric-conversion.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
