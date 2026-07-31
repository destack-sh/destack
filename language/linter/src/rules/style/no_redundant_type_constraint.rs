use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow constraints implied by other constraints.
    pub NO_REDUNDANT_TYPE_CONSTRAINT {
        id: "no-redundant-type-constraint",
        summary: "Disallow constraints implied by other constraints",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check no-redundant-type-constraint.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
