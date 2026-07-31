use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow shifts by amounts outside the operand's bit width.
    pub NO_OUT_OF_RANGE_SHIFT {
        id: "no-out-of-range-shift",
        summary: "Disallow shifts by amounts outside the operand's bit width",
        category: Correctness,
        level: Error,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check no-out-of-range-shift.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
