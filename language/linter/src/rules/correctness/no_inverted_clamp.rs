use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Disallow clamping with inverted bounds.
    pub NO_INVERTED_CLAMP {
        id: "no-inverted-clamp",
        summary: "Disallow clamping with inverted bounds",
        category: Correctness,
        level: Error,
        fixable: None,
        check: DirModule(check),
    }
}

/// Check no-inverted-clamp.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
