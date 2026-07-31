use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer the lowest set bit operation over its bitwise form.
    pub MANUAL_LOWEST_SET_BIT {
        id: "manual-lowest-set-bit",
        summary: "Prefer the lowest set bit operation over its bitwise form",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check manual-lowest-set-bit.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
