use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer the Euclidean remainder over its manual double-remainder form.
    pub MANUAL_EUCLIDEAN_REMAINDER {
        id: "manual-euclidean-remainder",
        summary: "Prefer the Euclidean remainder over its manual double-remainder form",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check manual-euclidean-remainder.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
