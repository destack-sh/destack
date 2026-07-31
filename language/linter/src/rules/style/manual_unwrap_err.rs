use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer unwrapErr over projecting the failed case manually.
    pub MANUAL_UNWRAP_ERR {
        id: "manual-unwrap-err",
        summary: "Prefer unwrapErr over projecting the failed case manually",
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Check manual-unwrap-err.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
