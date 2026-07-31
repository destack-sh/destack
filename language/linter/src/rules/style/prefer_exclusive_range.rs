use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer exclusive ranges over inclusive ranges offset by one.
    pub PREFER_EXCLUSIVE_RANGE {
        id: "prefer-exclusive-range",
        summary: "Prefer exclusive ranges over inclusive ranges offset by one",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check prefer-exclusive-range.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
