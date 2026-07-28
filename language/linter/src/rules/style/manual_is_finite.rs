use destack_repository::ProviderError;

use crate::rules::declare_lint_stub;
use crate::{DirModule, Lint, LintResult};

declare_lint_stub! {
    /// Prefer `.isFinite()` over equivalent infinity comparisons.
    pub MANUAL_IS_FINITE {
        id: "manual-is-finite",
        summary: "Prefer `.isFinite()` over equivalent infinity comparisons",
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Check manual-is-finite.
fn check(_module: &DirModule<'_>, lint: &Lint) -> LintResult {
    Err(ProviderError::internal(format!(
        "lint {} is not implemented",
        lint.id
    )))
}
