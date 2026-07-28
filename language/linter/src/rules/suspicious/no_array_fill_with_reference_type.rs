use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow Array.fill values whose type has reference identity.
    pub NO_ARRAY_FILL_WITH_REFERENCE_TYPE {
        id: "no-array-fill-with-reference-type",
        summary: "Disallow Array.fill values whose type has reference identity",
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check no-array-fill-with-reference-type.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
